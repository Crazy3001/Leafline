"""Writes the program's Tauri wrappers and the website's dispatcher for the shared commands.

Each command's body is a function in `crates/leafline-core/src/service/<module>.rs` that takes
`h: &dyn Host` first. This reads those signatures and writes:

- `src-tauri/src/app/generated/<module>.rs`: one `#[tauri::command]` per function, calling it with
  the program's `AppHost`;
- `crates/leafline-web/src/generated.rs`: `dispatch(name, args)` for the website's worker, which
  takes the same camelCase arguments the frontend passes to Tauri's `invoke`.

Run it from the repository root after changing a service function's signature:

    python tools/gen-commands.py

Both outputs are committed, so building needs no Python.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SERVICE = ROOT / 'crates/leafline-core/src/service'

# The service modules that hold commands, and the program module each one's wrappers belong to.
MODULES = {
    'builds': 'builds',
    'damage': 'damage',
    'mapdata': 'mapdata',
    'quests': 'questlog',
    'records': 'records',
    'sheet': 'sheet',
}

# Commands that download, which the program runs off its UI thread.
BLOCKING = {
    'download_map_data', 'download_travel', 'monster_drops', 'item_origin', 'quest_sheet',
    'monster_sheet', 'download_drops',
}

# Service functions taking a Host that are helpers, not commands.
NOT_COMMANDS = {'planned_gear', 'catch_up', 'note', 'set_buff_of', 'view', 'buffs_of', 'fighter', 'measured',
                'prior', 'csv', 'own_travel_file', 'travel_file'}


def split_args(text):
    """Splits `a: T, b: U<V, W>` at the top-level commas."""
    args, depth, cur = [], 0, ''
    for ch in text:
        if ch in '<([':
            depth += 1
        elif ch in '>)]':
            depth -= 1
        if ch == ',' and depth == 0:
            args.append(cur.strip())
            cur = ''
        else:
            cur += ch
    if cur.strip():
        args.append(cur.strip())
    return [tuple(part.strip() for part in a.split(':', 1)) for a in args]


def commands(module):
    text = (SERVICE / f'{module}.rs').read_text(encoding='utf-8')
    found = []
    for m in re.finditer(r'^pub fn (\w+)\(\s*h: &dyn Host,?([^{};]*?)\)\s*->\s*([^{};]+?)\s*\{\s*$', text, re.M):
        name, args, ret = m.group(1), m.group(2), m.group(3)
        if name in NOT_COMMANDS or '\n}' in args:
            continue
        found.append((name, split_args(' '.join(args.split())), ' '.join(ret.split())))
    return found


def imports(module):
    """The service module's own `use` lines, written from outside the core crate."""
    text = (SERVICE / f'{module}.rs').read_text(encoding='utf-8')
    lines = []
    for m in re.finditer(r'^(?:pub )?use [^;]+;', text, re.M | re.S):
        line = m.group(0)
        if line.startswith('pub '):
            line = line[4:]
        line = line.replace('use crate::', 'use leafline_core::').replace('use super::', 'use leafline_core::service::')
        lines.append(line)
    return lines


def camel(name):
    head, *rest = name.split('_')
    return head + ''.join(p.title() for p in rest)


def wrapper(module, name, args, ret):
    params = ''.join(f', {a}: {t}' for a, t in args)
    call = ', '.join(['&AppHost(app)'] + [a for a, _ in args])
    allow = '#[allow(clippy::too_many_arguments)]\n' if len(args) >= 6 else ''
    if name in BLOCKING:
        return (f'#[tauri::command]\n{allow}pub async fn {name}(app: AppHandle{params}) -> {ret} {{\n'
                f'    tauri::async_runtime::spawn_blocking(move || service::{module}::{name}({call}))\n'
                f'        .await\n        .map_err(|err| leafline_core::error::Error::Invalid(err.to_string()))?\n}}\n')
    return f'#[tauri::command]\n{allow}pub fn {name}(app: AppHandle{params}) -> {ret} {{\n    service::{module}::{name}({call})\n}}\n'


def program():
    out = ROOT / 'src-tauri/src/app/generated'
    out.mkdir(exist_ok=True)
    mods = []
    for module, app_module in MODULES.items():
        body = [
            f'//! Tauri commands for `leafline_core::service::{module}`. Written by tools/gen-commands.py;',
            '//! change the service function, then run the script again.',
            '#![allow(unused_imports)]',
            '',
            'use tauri::AppHandle;',
            '',
            'use leafline_core::service;',
            'use crate::app::host::AppHost;',
            f'use leafline_core::service::{module}::*;',
        ] + imports(module) + ['']
        for name, args, ret in commands(module):
            body.append(wrapper(module, name, args, ret))
        (out / f'{app_module}.rs').write_text('\n'.join(body), encoding='utf-8', newline='\n')
        mods.append(app_module)
    (out / 'mod.rs').write_text(
        '//! Tauri commands written by tools/gen-commands.py from the shared service functions.\n\n'
        + ''.join(f'pub mod {m};\n' for m in sorted(mods)), encoding='utf-8', newline='\n')


def web():
    out = ROOT / 'crates/leafline-web/src'
    if not out.exists():
        return
    parts, tries = [], []
    for module in MODULES:
        arms = []
        for name, args, ret in commands(module):
            send = 'reply_result' if ret.startswith(('Result<', 'crate::error::Result<')) else 'reply'
            if args:
                fields = ''.join(f'                {a}: {t},\n' for a, t in args)
                names = ', '.join(a for a, _ in args)
                call = ', '.join(['host'] + [a for a, _ in args])
                arms.append(
                    f'            "{name}" => {{\n'
                    f'                #[derive(serde::Deserialize)]\n'
                    f'                #[serde(rename_all = "camelCase")]\n'
                    f'                struct Args {{\n{fields}                }}\n'
                    f'                let Args {{ {names} }} = match parse(args) {{\n'
                    f'                    Ok(args) => args,\n'
                    f'                    Err(err) => return Some(Err(err)),\n'
                    f'                }};\n'
                    f'                {send}(service::{module}::{name}({call}))\n'
                    f'            }}\n')
            else:
                arms.append(f'            "{name}" => {send}(service::{module}::{name}(host)),\n')
        uses = '\n'.join('    ' + line.replace('\n', '\n    ') for line in imports(module))
        parts.append(
            f'mod {module} {{\n'
            f'    use leafline_core::service;\n'
            f'    use leafline_core::service::{module}::*;\n'
            f'{uses}\n\n'
            f'    use crate::reply::{{Reply, parse, reply, reply_result}};\n\n'
            f'    pub fn dispatch(host: &dyn Host, name: &str, args: serde_json::Value) -> Option<Reply> {{\n'
            f'        Some(match name {{\n' + ''.join(arms) + '            _ => return None,\n        })\n    }\n}\n')
        tries.append(f'        .or_else(|| {module}::dispatch(host, name, args.clone()))\n')
    text = (
        '//! The website\'s command dispatcher. Written by tools/gen-commands.py; change the service\n'
        '//! function, then run the script again.\n'
        '#![allow(unused_imports, clippy::too_many_lines)]\n\n'
        'use leafline_core::service::host::Host;\n\n'
        'use crate::reply::Reply;\n\n'
        '/// Runs a shared command by the name the program\'s frontend invokes it by. `None` when the\n'
        '/// name isn\'t one of them.\n'
        'pub fn dispatch(host: &dyn Host, name: &str, args: serde_json::Value) -> Option<Reply> {\n'
        '    None' + ''.join(tries) + '}\n\n' + '\n'.join(parts))
    (out / 'generated.rs').write_text(text, encoding='utf-8', newline='\n')


if __name__ == '__main__':
    program()
    web()
    print('written')

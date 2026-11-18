//! Crafting recipes from the map data: which recipe makes an item and which recipes use it, for
//! the item page, and the whole list by discipline for the Crafting section. Names come from the
//! item data; the recipes file only holds ids.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::catalog::Catalog;
use crate::mapdata::osms::RecipeRecord;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemAmount {
    pub id: u32,
    /// The item's name, or its id written out when the item data doesn't know it.
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub id: u32,
    pub discipline: String,
    /// The discipline's skill level the recipe asks for.
    pub skill_level: u32,
    pub craft_exp: u32,
    pub meso_cost: u64,
    pub output: ItemAmount,
    pub ingredients: Vec<ItemAmount>,
}

/// An item's place in crafting.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemCrafting {
    /// Recipes that make the item.
    pub made_by: Vec<Recipe>,
    /// Recipes that take it as an ingredient.
    pub used_in: Vec<Recipe>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CraftingQuery {
    pub discipline: Option<String>,
    /// Matches the item made or any ingredient.
    pub search: Option<String>,
    /// Recipes at or below this skill level.
    pub max_level: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CraftingPage {
    /// Every discipline and how many recipes it has, by name.
    pub disciplines: Vec<(String, u32)>,
    /// The recipes that pass the query, by discipline, then skill level, then name.
    pub recipes: Vec<Recipe>,
}

impl Catalog {
    fn amount(&self, id: u32, count: u32) -> ItemAmount {
        let name = self.item(id).map_or_else(|| format!("Item {id}"), |i| i.name.clone());
        ItemAmount { id, name, count }
    }

    fn recipe(&self, r: &RecipeRecord) -> Recipe {
        Recipe {
            id: r.id,
            discipline: r.discipline.clone(),
            skill_level: r.skill_level,
            craft_exp: r.craft_exp,
            meso_cost: r.meso_cost,
            output: self.amount(r.output_item_id, r.output_count),
            ingredients: r.ingredients.iter().map(|g| self.amount(g.item_id, g.count)).collect(),
        }
    }

    fn ordered(&self, mut recipes: Vec<Recipe>) -> Vec<Recipe> {
        recipes.sort_by(|a, b| {
            a.discipline
                .cmp(&b.discipline)
                .then(a.skill_level.cmp(&b.skill_level))
                .then_with(|| a.output.name.cmp(&b.output.name))
                .then(a.id.cmp(&b.id))
        });
        recipes
    }

    /// The recipes that make an item and the ones that use it, over all of the item's copies.
    pub fn crafting_of(&self, id: u32) -> ItemCrafting {
        let ids = self.copies_of(id);
        let made_by = self.recipe_list().iter().filter(|r| ids.contains(&r.output_item_id)).map(|r| self.recipe(r)).collect();
        let used_in = self
            .recipe_list()
            .iter()
            .filter(|r| r.ingredients.iter().any(|g| ids.contains(&g.item_id)))
            .map(|r| self.recipe(r))
            .collect();
        ItemCrafting { made_by: self.ordered(made_by), used_in: self.ordered(used_in) }
    }

    /// The Crafting section's list.
    pub fn crafting(&self, query: &CraftingQuery) -> CraftingPage {
        let mut disciplines: BTreeMap<String, u32> = BTreeMap::new();
        for r in self.recipe_list() {
            *disciplines.entry(r.discipline.clone()).or_default() += 1;
        }
        let search = query.search.as_deref().map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());
        let recipes = self
            .recipe_list()
            .iter()
            .filter(|r| query.discipline.as_deref().is_none_or(|d| r.discipline.eq_ignore_ascii_case(d)))
            .filter(|r| query.max_level.is_none_or(|max| r.skill_level <= max))
            .map(|r| self.recipe(r))
            .filter(|r| {
                search.as_deref().is_none_or(|s| {
                    r.output.name.to_lowercase().contains(s) || r.ingredients.iter().any(|g| g.name.to_lowercase().contains(s))
                })
            })
            .collect();
        CraftingPage { disciplines: disciplines.into_iter().collect(), recipes: self.ordered(recipes) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::catalog::tests::item;
    use crate::mapdata::osms::Ingredient;

    // Made-up recipes over made-up items.
    fn workshop() -> Catalog {
        let recipe = |id, discipline: &str, level, output, count, ingredients: &[(u32, u32)]| RecipeRecord {
            id,
            discipline: discipline.into(),
            skill_level: level,
            craft_exp: 3,
            meso_cost: 100,
            output_item_id: output,
            output_count: count,
            ingredients: ingredients.iter().map(|&(item_id, count)| Ingredient { item_id, count }).collect(),
        };
        Catalog::new(
            Vec::new(),
            vec![item(1, "Pebble Ore", 0, &[]), item(2, "Pebble Ingot", 0, &[]), item(3, "Pebble Helm", 10, &[])],
        )
        .with_recipes(vec![
            recipe(10, "Stoneworking", 2, 3, 1, &[(2, 4), (99, 1)]),
            recipe(11, "Stoneworking", 1, 2, 1, &[(1, 5)]),
            recipe(12, "Gluing", 1, 2, 2, &[(1, 8)]),
        ])
    }

    #[test]
    fn an_item_knows_what_makes_it_and_what_it_goes_into() {
        let crafting = workshop().crafting_of(2);
        let made: Vec<_> = crafting.made_by.iter().map(|r| (r.discipline.as_str(), r.output.count)).collect();
        assert_eq!(made, [("Gluing", 2), ("Stoneworking", 1)]);
        assert_eq!(crafting.used_in.len(), 1);
        let helm = &crafting.used_in[0];
        assert_eq!(helm.output.name, "Pebble Helm");
        assert_eq!(helm.ingredients[0], ItemAmount { id: 2, name: "Pebble Ingot".into(), count: 4 });
        assert_eq!(helm.ingredients[1].name, "Item 99", "an item the data lacks is named by its id");
        assert!(workshop().crafting_of(3).used_in.is_empty());
    }

    #[test]
    fn the_list_filters_by_discipline_level_and_name() {
        let catalog = workshop();
        let all = catalog.crafting(&CraftingQuery::default());
        assert_eq!(all.disciplines, [("Gluing".to_string(), 1), ("Stoneworking".to_string(), 2)]);
        let ids = |page: CraftingPage| page.recipes.into_iter().map(|r| r.id).collect::<Vec<_>>();
        assert_eq!(ids(all), [12, 11, 10]);
        let stone = CraftingQuery { discipline: Some("stoneworking".into()), max_level: Some(1), ..CraftingQuery::default() };
        assert_eq!(ids(catalog.crafting(&stone)), [11]);
        let ore = CraftingQuery { search: Some(" ore ".into()), ..CraftingQuery::default() };
        assert_eq!(ids(catalog.crafting(&ore)), [12, 11], "matched on an ingredient");
    }
}

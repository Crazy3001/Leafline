// Shapes the chart components take.

export interface ChartPoint {
  /** Active seconds into the session. */
  x: number;
  y: number | null;
}

export interface Bar {
  id: string;
  label: string;
  sub?: string;
  value: number;
  display: string;
  /** Rests on too little data to trust; drawn lighter. */
  faded?: boolean;
}

export interface Column {
  label: string;
  value: number;
  display: string;
  detail: string;
}

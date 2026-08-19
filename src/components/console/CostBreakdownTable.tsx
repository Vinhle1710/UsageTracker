import type { CostPoint } from "../../types";
export function CostBreakdownTable({ title, points }: { title: string; points: CostPoint[] }) {
  const sorted=[...points].sort((a,b)=>{ const left=BigInt(a.amount.minorUnits), right=BigInt(b.amount.minorUnits); return left===right?0:left>right?-1:1; });
  const format=(p:CostPoint)=>new Intl.NumberFormat(undefined,{style:"currency",currency:p.amount.currency}).format(Number(BigInt(p.amount.minorUnits))/100);
  return <table><caption>{title}</caption><thead><tr><th scope="col">Name</th><th scope="col">Amount</th></tr></thead><tbody>{sorted.map((p)=><tr key={p.key}><th scope="row">{p.label}</th><td>{format(p)}</td></tr>)}</tbody></table>;
}

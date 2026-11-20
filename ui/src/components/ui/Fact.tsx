export function Fact({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return <div><dt>{label}</dt><dd className={mono ? 'mono' : undefined} title={mono ? value : undefined}>{value}</dd></div>
}

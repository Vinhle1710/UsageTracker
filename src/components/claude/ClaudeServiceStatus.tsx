import type { ClaudeServiceStatus as Status } from "../../types";
export function ClaudeServiceStatus({ status }: { status: Status | null | undefined }) {
 if (!status) return <p role="status">Claude service status unavailable</p>;
 return <section aria-label="Claude service status"><p role="status"><span aria-hidden="true">●</span> {status.indicator}: {status.description}</p><ul>{status.incidents.map((incident) => <li key={incident.name}>{incident.name} ({incident.status}) {incident.url ? <a href={incident.url} target="_blank" rel="noreferrer">Details</a> : null}</li>)}</ul></section>;
}

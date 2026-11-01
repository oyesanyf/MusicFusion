// c8: In-loop protocol framing enforcement plugin
export function enforceStrictCRLF(payload: string): boolean {
  return payload.includes('\r\n\r\n');
}

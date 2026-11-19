/** A round initial badge. The tone comes from the name, so a person keeps their colour. */
export function toneFor(name: string): number {
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.codePointAt(0)!) >>> 0;
  return h % 4;
}

export function initialOf(name: string): string {
  const first = [...name.trim()][0];
  return first ? first.toLocaleUpperCase() : "?";
}

export function Avatar({ name }: { name: string }) {
  return (
    <span className="avatar" data-tone={toneFor(name)} aria-hidden="true">
      {initialOf(name)}
    </span>
  );
}

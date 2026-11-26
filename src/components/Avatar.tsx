/** A round initial badge, grey for everyone, like a monogram in Contacts. */
export function initialOf(name: string): string {
  const first = [...name.trim()][0];
  return first ? first.toLocaleUpperCase() : "?";
}

export function Avatar({ name }: { name: string }) {
  return (
    <span className="avatar" aria-hidden="true">
      {initialOf(name)}
    </span>
  );
}

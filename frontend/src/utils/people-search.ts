export type PersonRole = "all" | "owner" | "administrator" | "member";
export type PersonSort = "newest" | "oldest" | "name" | "email";

export function personMatches(person: { email: string; displayName: string }, query: string): boolean {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    return true;
  }
  return person.email.toLowerCase().includes(needle) || person.displayName.toLowerCase().includes(needle);
}

export function selectPeople<
  T extends { email: string; displayName: string; roles: string[]; createdAt: string },
>(items: T[], query: string, role: PersonRole, sort: PersonSort): T[] {
  const matched = items.filter((person) => {
    if (!personMatches(person, query)) {
      return false;
    }
    if (role === "all") {
      return true;
    }
    return person.roles.includes(role);
  });
  return matched.sort((left, right) => {
    if (sort === "name") {
      return left.displayName.localeCompare(right.displayName);
    }
    if (sort === "email") {
      return left.email.localeCompare(right.email);
    }
    const byTime = left.createdAt.localeCompare(right.createdAt);
    return sort === "oldest" ? byTime : -byTime;
  });
}

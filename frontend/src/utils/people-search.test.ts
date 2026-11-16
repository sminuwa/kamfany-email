import { describe, expect, it } from "vitest";
import { personMatches, selectPeople } from "./people-search";

const ada = {
  email: "adaeze@kamfany.com",
  displayName: "Adaeze Okonkwo",
  roles: ["owner"],
  createdAt: "2026-01-01T00:00:00Z",
};

describe("people search", () => {
  it("keeps every account when the search is empty", () => {
    expect(personMatches(ada, "  ")).toBe(true);
  });

  it("matches the email or the display name", () => {
    expect(personMatches(ada, "ADAEZE@")).toBe(true);
    expect(personMatches(ada, "okonkwo")).toBe(true);
    expect(personMatches(ada, "finance")).toBe(false);
  });

  it("filters by role and sorts newest first", () => {
    const rows = [
      ada,
      {
        email: "member@kamfany.com",
        displayName: "Member",
        roles: ["member"],
        createdAt: "2026-03-01T00:00:00Z",
      },
    ];
    expect(selectPeople(rows, "", "owner", "newest").map((row) => row.email)).toEqual(["adaeze@kamfany.com"]);
    expect(selectPeople(rows, "", "all", "newest").map((row) => row.email)).toEqual([
      "member@kamfany.com",
      "adaeze@kamfany.com",
    ]);
    expect(selectPeople(rows, "", "all", "name").map((row) => row.displayName)).toEqual(["Adaeze Okonkwo", "Member"]);
  });
});
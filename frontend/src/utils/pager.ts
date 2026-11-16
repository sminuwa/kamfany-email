export const PAGE_SIZE = 10;

export function pageCount(total: number, size = PAGE_SIZE): number {
  if (total <= 0) {
    return 1;
  }
  return Math.ceil(total / size);
}

export function pageSlice<T>(items: T[], page: number, size = PAGE_SIZE): T[] {
  const current = Math.min(Math.max(page, 1), pageCount(items.length, size));
  const start = (current - 1) * size;
  return items.slice(start, start + size);
}

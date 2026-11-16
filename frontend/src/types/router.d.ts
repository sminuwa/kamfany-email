export {};

declare module "vue-router" {
  interface RouteMeta {
    folder?: string;
    title?: string;
    permission?: string;
  }
}

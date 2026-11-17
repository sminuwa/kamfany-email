import { mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { describe, expect, it } from "vitest";
import { createMemoryHistory, createRouter } from "vue-router";
import { systemFolders } from "../../utils/folders";
import AppSidebar from "./AppSidebar.vue";

describe("AppSidebar", () => {
  it("shows the product name, compose, and every default folder", async () => {
    const router = createRouter({
      history: createMemoryHistory(),
      routes: systemFolders.map((folder) => ({
        path: folder.path,
        component: { template: "<div />" },
      })),
    });
    const wrapper = mount(AppSidebar, {
      global: {
        plugins: [createPinia(), router],
      },
    });
    await router.isReady();

    expect(wrapper.text()).toContain("Mail");
    expect(wrapper.text()).toContain("Compose");
    for (const folder of systemFolders) {
      expect(wrapper.text()).toContain(folder.label);
    }
    expect(wrapper.get("a[href='/']").attributes("aria-current")).toBe("page");
  });
});

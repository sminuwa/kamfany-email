import { createRouter, createWebHistory } from "vue-router";
import AuthLayout from "../layouts/AuthLayout.vue";
import MailLayout from "../layouts/MailLayout.vue";
import AccessView from "../pages/auth/AccessView.vue";
import ContactsView from "../pages/contacts/ContactsView.vue";
import DeskView from "../pages/mail/DeskView.vue";
import { useAuthStore } from "../stores/auth";
import { useMailboxStore } from "../stores/mailboxes";
import { folderByKey, systemFolders } from "../utils/folders";

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      component: MailLayout,
      children: [
        ...systemFolders.map((folder) => ({
          path: folder.path === "/" ? "" : folder.path.slice(1),
          name: folder.key,
          component: DeskView,
          meta: { folder: folder.key, title: folder.label },
        })),
        {
          path: "folders/:folderId",
          name: "folder",
          component: DeskView,
          meta: { title: "Folder" },
        },
        {
          path: "contacts",
          name: "contacts",
          component: ContactsView,
          meta: { title: "Contacts" },
        },
      ],
    },
    {
      path: "/account",
      component: AuthLayout,
      children: [
        {
          path: "",
          name: "account",
          component: AccessView,
          meta: { title: "Account" },
        },
      ],
    },
  ],
});

router.beforeEach(async (to) => {
  const auth = useAuthStore();
  await auth.ensure();
  if (auth.status !== "signed_in") {
    if (to.path === "/account") {
      return true;
    }
    return { path: "/account", query: { next: to.fullPath } };
  }
  if (to.path !== "/account") {
    await useMailboxStore().ensure();
  }
  return true;
});

router.afterEach((to) => {
  const folder = to.meta.folder ? folderByKey(to.meta.folder) : undefined;
  const title = to.meta.title ?? folder?.label;
  document.title = title ? `${title} · KAMFANY Mail` : "KAMFANY Mail";
});

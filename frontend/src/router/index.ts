import { createRouter, createWebHistory } from "vue-router";
import AuthLayout from "../layouts/AuthLayout.vue";
import MailLayout from "../layouts/MailLayout.vue";
import AccessView from "../pages/auth/AccessView.vue";
import AuditView from "../pages/auth/AuditView.vue";
import MailboxesView from "../pages/auth/MailboxesView.vue";
import PasswordView from "../pages/auth/PasswordView.vue";
import PeopleView from "../pages/auth/PeopleView.vue";
import ProviderView from "../pages/auth/ProviderView.vue";
import ResetPasswordView from "../pages/auth/ResetPasswordView.vue";
import SessionsView from "../pages/auth/SessionsView.vue";
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
          meta: { title: "Account settings" },
        },
        {
          path: "password",
          name: "account-password",
          component: PasswordView,
          meta: { title: "Change password" },
        },
        {
          path: "sessions",
          name: "account-sessions",
          component: SessionsView,
          meta: { title: "Sessions" },
        },
        {
          path: "mailboxes",
          name: "account-mailboxes",
          component: MailboxesView,
          meta: { title: "Mailboxes" },
        },
        {
          path: "mailboxes/new",
          redirect: { name: "account-mailboxes" },
        },
        {
          path: "mailboxes/access",
          redirect: (to) => ({ name: "account-mailboxes", query: to.query }),
        },
        {
          path: "provider",
          name: "account-provider",
          component: ProviderView,
          meta: { title: "Email provider", permission: "mailbox.manage" },
        },
        {
          path: "audit",
          name: "account-audit",
          component: AuditView,
          meta: { title: "Audit log", permission: "audit.read" },
        },
        {
          path: "people",
          name: "account-people",
          component: PeopleView,
          meta: { title: "People", permission: "users.manage" },
        },
        {
          path: "people/new",
          redirect: { name: "account-people" },
        },
      ],
    },
    {
      path: "/reset-password",
      component: AuthLayout,
      children: [
        {
          path: "",
          name: "reset-password",
          component: ResetPasswordView,
          meta: { title: "Reset password" },
        },
      ],
    },
  ],
});

router.beforeEach(async (to) => {
  const auth = useAuthStore();
  await auth.ensure();
  if (auth.status !== "signed_in") {
    if (to.path === "/account" || to.path === "/reset-password") {
      return true;
    }
    return { path: "/account", query: { next: to.fullPath } };
  }
  const permission = typeof to.meta.permission === "string" ? to.meta.permission : "";
  if (permission && !auth.user?.permissions.includes(permission)) {
    return { path: "/account" };
  }
  if (!to.path.startsWith("/account")) {
    await useMailboxStore().ensure();
  }
  return true;
});

router.afterEach((to) => {
  const folder = to.meta.folder ? folderByKey(to.meta.folder) : undefined;
  const title = to.meta.title ?? folder?.label;
  document.title = title ? `${title} · KAMFANY Mail` : "KAMFANY Mail";
});

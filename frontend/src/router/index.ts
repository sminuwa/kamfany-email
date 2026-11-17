import { createRouter, createWebHistory } from "vue-router";
import AuthLayout from "../layouts/AuthLayout.vue";
import MailLayout from "../layouts/MailLayout.vue";
import AccessView from "../pages/auth/AccessView.vue";
import GroupsView from "../pages/auth/GroupsView.vue";
import AuditView from "../pages/auth/AuditView.vue";
import PasswordView from "../pages/auth/PasswordView.vue";
import PeopleView from "../pages/auth/PeopleView.vue";
import ProviderView from "../pages/auth/ProviderView.vue";
import ResetPasswordView from "../pages/auth/ResetPasswordView.vue";
import SessionsView from "../pages/auth/SessionsView.vue";
import ContactsView from "../pages/contacts/ContactsView.vue";
import DeskView from "../pages/mail/DeskView.vue";
import { useAuthStore } from "../stores/auth";
import { useBrandStore } from "../stores/brand";
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
        {
          path: "account",
          name: "account",
          component: AccessView,
          meta: { title: "Account settings" },
        },
        {
          path: "account/password",
          name: "account-password",
          component: PasswordView,
          meta: { title: "Change password" },
        },
        {
          path: "account/sessions",
          name: "account-sessions",
          component: SessionsView,
          meta: { title: "Sessions" },
        },
        {
          path: "account/mailboxes",
          name: "account-mailboxes",
          component: PeopleView,
          meta: { title: "Mailboxes", permission: "users.manage" },
        },
        {
          path: "account/mailboxes/new",
          redirect: { name: "account-mailboxes" },
        },
        {
          path: "account/mailboxes/access",
          redirect: { name: "account-mailboxes" },
        },
        {
          path: "account/provider",
          name: "account-provider",
          component: ProviderView,
          meta: { title: "Email provider", permission: "mailbox.manage" },
        },
        {
          path: "account/groups",
          name: "account-groups",
          component: GroupsView,
          meta: { title: "Groups", role: "owner" },
        },
        {
          path: "account/audit",
          name: "account-audit",
          component: AuditView,
          meta: { title: "Audit log", permission: "audit.read" },
        },
        {
          path: "account/people",
          redirect: { name: "account-mailboxes" },
        },
        {
          path: "account/people/new",
          redirect: { name: "account-mailboxes" },
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
  const role = typeof to.meta.role === "string" ? to.meta.role : "";
  if (role && !auth.user?.roles.includes(role)) {
    return { path: "/account" };
  }
  await useMailboxStore().ensure();
  return true;
});

router.afterEach((to) => {
  const folder = to.meta.folder ? folderByKey(to.meta.folder) : undefined;
  const title = to.meta.title ?? folder?.label;
  const brand = useBrandStore().name;
  document.title = title ? `${title} · ${brand}` : brand;
});

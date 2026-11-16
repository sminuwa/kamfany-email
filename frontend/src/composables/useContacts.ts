import { onMounted, reactive, ref, watch } from "vue";
import {
  createContact,
  deleteAvatar,
  deleteContact,
  emptyContact,
  fetchContacts,
  importContacts,
  saveContact,
  uploadAvatar,
  type Contact,
  type ContactDraft,
} from "../services/contact.service";
import { errorMessage } from "../services/http";

export function useContacts() {
  const people = ref<Contact[]>([]);
  const query = ref("");
  const selectedId = ref<string | null>(null);
  const draft = ref<ContactDraft>(emptyContact());
  const hasAvatar = ref(false);
  const updatedAt = ref("");
  const failure = ref("");
  const notice = ref("");
  const saving = ref(false);
  let timer = 0;

  async function load() {
    try {
      people.value = await fetchContacts(query.value);
      failure.value = "";
    } catch (error) {
      failure.value = errorMessage(error);
    }
  }

  function choose(person: Contact, keepNotice = false) {
    selectedId.value = person.id;
    hasAvatar.value = person.hasAvatar;
    updatedAt.value = person.updatedAt;
    draft.value = {
      fullName: person.fullName,
      email: person.email,
      phone: person.phone,
      company: person.company,
      notes: person.notes,
    };
    failure.value = "";
    if (!keepNotice) {
      notice.value = "";
    }
  }

  function startNew() {
    selectedId.value = null;
    hasAvatar.value = false;
    updatedAt.value = "";
    draft.value = emptyContact();
    failure.value = "";
    notice.value = "";
  }

  async function save() {
    saving.value = true;
    failure.value = "";
    try {
      const saved = selectedId.value
        ? await saveContact(selectedId.value, draft.value)
        : await createContact(draft.value);
      notice.value = "Contact saved";
      await load();
      choose(saved, true);
    } catch (error) {
      failure.value = errorMessage(error);
    } finally {
      saving.value = false;
    }
  }

  async function remove() {
    if (!selectedId.value) {
      return;
    }
    failure.value = "";
    try {
      await deleteContact(selectedId.value);
      startNew();
      notice.value = "Contact deleted";
      await load();
    } catch (error) {
      failure.value = errorMessage(error);
    }
  }

  async function setAvatar(file: File | null) {
    if (!file || !selectedId.value) {
      return;
    }
    failure.value = "";
    try {
      const saved = await uploadAvatar(selectedId.value, file);
      choose(saved);
      await load();
    } catch (error) {
      failure.value = errorMessage(error);
    }
  }

  async function clearAvatar() {
    if (!selectedId.value) {
      return;
    }
    failure.value = "";
    try {
      await deleteAvatar(selectedId.value);
      hasAvatar.value = false;
      await load();
    } catch (error) {
      failure.value = errorMessage(error);
    }
  }

  async function importFile(file: File | null) {
    if (!file) {
      return;
    }
    failure.value = "";
    try {
      const result = await importContacts(file);
      notice.value = `Imported ${result.created} new, ${result.updated} updated, ${result.skipped} skipped.`;
      await load();
    } catch (error) {
      failure.value = errorMessage(error);
    }
  }

  watch(query, () => {
    window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      void load();
    }, 250);
  });

  onMounted(() => {
    void load();
  });

  return reactive({
    people,
    query,
    selectedId,
    draft,
    hasAvatar,
    updatedAt,
    failure,
    notice,
    saving,
    choose,
    startNew,
    save,
    remove,
    setAvatar,
    clearAvatar,
    importFile,
  });
}

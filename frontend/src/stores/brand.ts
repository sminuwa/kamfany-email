import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { fetchBrand } from "../services/brand.service";

export const useBrandStore = defineStore("brand", () => {
  const name = ref("Mail");
  const company = ref("");
  const domain = ref("");
  const publicUrl = ref("");
  const logoUrl = ref("");

  const byline = computed(() =>
    company.value ? `${name.value} for ${company.value}.` : name.value,
  );
  const addressHint = computed(() =>
    domain.value ? `name@${domain.value}` : "name@example.com",
  );
  const personHint = computed(() =>
    domain.value ? `person@${domain.value}` : "person@example.com",
  );

  function applyDocument() {
    document.title = name.value;
    document
      .querySelector('meta[name="description"]')
      ?.setAttribute("content", byline.value);
  }

  async function load() {
    try {
      const profile = await fetchBrand();
      if (profile.name.trim()) {
        name.value = profile.name.trim();
      }
      company.value = profile.company.trim();
      domain.value = profile.domain.trim();
      publicUrl.value = profile.publicUrl.trim();
      logoUrl.value = profile.logoUrl.trim();
    } catch {
      // The desk still opens with the neutral name when the API is down.
    }
    applyDocument();
  }

  return {
    name,
    company,
    domain,
    publicUrl,
    logoUrl,
    byline,
    addressHint,
    personHint,
    load,
  };
});

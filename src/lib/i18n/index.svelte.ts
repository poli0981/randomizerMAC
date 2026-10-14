import { en, type MessageKey } from "./en";
import { vi } from "./vi";

export type Locale = "en" | "vi";
export type LanguageSetting = "system" | Locale;

const dictionaries = { en, vi } as const;

function systemLocale(): Locale {
  const langs = typeof navigator !== "undefined" ? navigator.languages : [];
  return langs.some((l) => l.toLowerCase().startsWith("vi")) ? "vi" : "en";
}

class I18n {
  locale = $state<Locale>(systemLocale());

  setLanguage(setting: LanguageSetting) {
    this.locale = setting === "system" ? systemLocale() : setting;
    if (typeof document !== "undefined") {
      document.documentElement.lang = this.locale;
    }
  }

  /** Translates `key`, replacing `{name}` placeholders. */
  t = (key: MessageKey, params?: Record<string, string | number>): string => {
    const template: string = dictionaries[this.locale][key] ?? en[key] ?? key;
    if (!params) return template;
    return template.replace(/\{(\w+)\}/g, (match, name: string) =>
      name in params ? String(params[name]) : match,
    );
  };

  /** Translates an error code coming from the backend. */
  error = (code: string | undefined | null): string => {
    const key = `error.${code}` as MessageKey;
    if (code && key in en) return this.t(key);
    return this.t("error.unknown", { code: code ?? "?" });
  };
}

export const i18n = new I18n();
export const t = i18n.t;
export type { MessageKey };

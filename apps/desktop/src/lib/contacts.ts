import { getLocale } from "./i18n";
import { AsYouType, isSupportedCountry } from "libphonenumber-js/max";
import { normalizeDialNumber } from "./dialing";
import type {
  AppLocale,
  Contact,
  ContactPhone,
  ContactsPreferences,
} from "./generated/AppInfo";

export const defaultNumberRegion = "DE";

/** Missing region metadata from older versions keeps the original German default. */
export function matchingNumberRegion(
  preferences: Pick<ContactsPreferences, "matchGermanNumbers"> &
    Partial<Pick<ContactsPreferences, "numberRegion">>,
): string | null {
  return preferences.matchGermanNumbers === false
    ? null
    : (preferences.numberRegion ?? defaultNumberRegion);
}

function searchable(value: string): string {
  return value
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/ß/g, "ss");
}

function phoneSearch(value: string): string | null {
  if (!/^[0-9+*#()\s./-]+$/.test(value)) return null;
  const digits = value.replace(/[^0-9+*#]/g, "");
  return digits || null;
}

/** Filter and sort contacts without merging namesakes or mutating the source. */
export function searchContacts(
  contacts: readonly Contact[],
  query: string,
  favoritesOnly = false,
  favoritesFirst = true,
  numberRegion: string | null = defaultNumberRegion,
  language: AppLocale = getLocale(),
): Contact[] {
  const text = searchable(query.trim());
  const telephone = phoneSearch(text);
  const terms = text.split(/\s+/).filter(Boolean);
  const dialQuery = telephone ? normalizeDialNumber(query) : null;
  const queryKey = dialQuery ? numberKey(dialQuery, numberRegion, true) : null;
  return contacts
    .filter((contact) => {
      if (favoritesOnly && !contact.favorite) return false;
      const fields = searchable(`${contact.name} ${contact.organization}`);
      const numbers = contact.phones.map(
        (phone) => phoneSearch(phone.number) ?? searchable(phone.number),
      );
      if (telephone)
        return (
          fields.includes(text) ||
          numbers.some((number) => number.includes(telephone)) ||
          contact.phones.some((phone) => {
            const value = normalizeDialNumber(phone.number);
            const key = value && numberKey(value, numberRegion);
            return !!key && !!queryKey && key.includes(queryKey);
          })
        );
      return terms.every(
        (term) =>
          fields.includes(term) ||
          numbers.some((number) => number.includes(phoneSearch(term) ?? term)),
      );
    })
    .sort(
      (a, b) =>
        (favoritesFirst ? Number(b.favorite) - Number(a.favorite) : 0) ||
        a.name.localeCompare(b.name, language, { sensitivity: "base" }) ||
        a.id.localeCompare(b.id),
    );
}

/** Compare lookup keys while the actual dial destinations retain their source values. */
function numberKey(
  value: string,
  region: string | null,
  partial = false,
): string {
  if (!/^\d+$/.test(value)) return value;
  if (value.startsWith("00"))
    return value.length >= (partial ? 4 : 7) ? `+${value.slice(2)}` : value;
  if (!region || !isSupportedCountry(region)) return value;

  const parser = new AsYouType(region);
  parser.input(value);
  if (parser.isInternational() || (!partial && !parser.isValid())) return value;
  return parser.getNumberValue() ?? value;
}
export interface DialSuggestion {
  contact: Contact;
  phone: ContactPhone;
}
/** Suggestions are choices, not authenticated identities; ambiguous owners remain separate. */
export function dialSuggestions(
  contacts: readonly Contact[],
  number: string,
  numberRegion: string | null = defaultNumberRegion,
  language: AppLocale = getLocale(),
): DialSuggestion[] {
  const query = normalizeDialNumber(number);
  if (!query || query.replace(/\D/g, "").length < 3) return [];
  const prefix = numberKey(query, numberRegion, true);
  const candidates = contacts
    .flatMap((contact) =>
      contact.phones.flatMap((phone) => {
        if (!phone.destination) return [];
        const value = normalizeDialNumber(phone.number);
        if (!value) return [];
        const key = numberKey(value, numberRegion);
        return value.startsWith(query) || key.startsWith(prefix)
          ? [{ contact, phone, exactMatch: value === query || key === prefix }]
          : [];
      }),
    )
    .sort(
      (a, b) =>
        Number(b.exactMatch) - Number(a.exactMatch) ||
        Number(b.contact.favorite) - Number(a.contact.favorite) ||
        a.contact.name.localeCompare(b.contact.name, language),
    );
  const suggestions: DialSuggestion[] = [];
  for (const { contact, phone } of candidates) {
    if (
      suggestions.some(
        (other) =>
          other.contact.id === contact.id &&
          other.phone.destination === phone.destination,
      )
    )
      continue;
    suggestions.push({ contact, phone });
    if (suggestions.length === 3) break;
  }
  return suggestions;
}

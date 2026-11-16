import type { ReactNode } from "react";
import { X } from "lucide-react";
import type { Language } from "../model";
import { translate as t } from "../i18n";

type Props = { children: ReactNode; language: Language; dismiss: () => void };

export function DismissibleNotice({ children, language, dismiss }: Props) {
  return <div className="notice dismissible-notice"><span>{children}</span><button className="icon-button" title={t(language, "dismissNotice")} aria-label={t(language, "dismissNotice")} onClick={dismiss}><X size={17}/></button></div>;
}

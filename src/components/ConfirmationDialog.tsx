import { useEffect, useRef } from "react";
import { ShieldAlert } from "lucide-react";
import type { Language } from "../model";
import { translate as t } from "../i18n";

type Props = { message: string; language: Language; onDecision: (confirmed: boolean) => void };

export function ConfirmationDialog({ message, language, onDecision }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const cancel = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    element.showModal();
    cancel.current?.focus();
    return () => { if (element.open) element.close(); };
  }, []);

  function decide(confirmed: boolean) {
    dialog.current?.close();
    onDecision(confirmed);
  }

  return <dialog ref={dialog} className="confirm-dialog" aria-labelledby="confirmation-title" aria-describedby="confirmation-message" onCancel={(event) => { event.preventDefault(); decide(false); }} onKeyDown={(event) => { if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); decide(false); } }}>
    <div className="confirm-heading"><span className="confirm-icon"><ShieldAlert size={20}/></span><div><span className="eyebrow">GAME OPTIMIZER</span><h2 id="confirmation-title">{t(language, "confirmationTitle")}</h2></div></div>
    <p id="confirmation-message" className="confirm-message scroll-panel" tabIndex={0}>{message}</p>
    <div className="modal-actions"><button ref={cancel} className="secondary" onClick={() => decide(false)}>{t(language, "cancel")}</button><button className="primary" onClick={() => decide(true)}>{t(language, "confirmAction")}</button></div>
  </dialog>;
}

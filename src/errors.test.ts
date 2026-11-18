import { describe, expect, it } from "vitest";
import { applyWarningText, localizeError, skippedText } from "./errors";

describe("restore warnings", () => {
  it("distinguishes external conflicts from retryable failures in both languages", () => {
    const warnings = [
      { step: "gpu", message: "GPU preference was modified externally", retryable: false },
      { step: "cpu", message: "Access denied", retryable: true },
    ];
    expect(skippedText("es", warnings)).toContain("se modificó fuera de la aplicación");
    expect(skippedText("es", warnings)).toContain("vuelve a restaurar");
    expect(skippedText("en", warnings)).toContain("changed outside the app");
    expect(skippedText("en", warnings)).toContain("retry restoration");
  });

  describe("operation errors", () => {
    it("formats each skipped optimization step separately in both languages", () => {
      const warnings = [
        { step: "defender-path", message: "Defender exclusion overlaps another active profile", recovery_required: false },
        { step: "defender-process", message: "Defender process exclusion belongs to another active profile", recovery_required: false },
      ];
      const es = warnings.map((warning) => applyWarningText("es", warning));
      expect(es[0]).toBe("Se omitió la exclusión de carpeta de Defender. La carpeta se superpone con otra reserva de Defender.");
      expect(es[0]).not.toContain("Exclusión de proceso de Defender:");
      expect(es[1]).toContain("Otro perfil ya administra la exclusión de proceso");
      const en = warnings.map((warning) => applyWarningText("en", warning));
      expect(en[0]).toBe("Defender folder exclusion skipped. The folder overlaps another Defender reservation.");
      expect(en[1]).toContain("Another profile already manages this executable's Defender process exclusion");
    });

    it("identifies the Defender step, overlap and recovery in both languages", () => {
      const detail = "defender-path: Defender exclusion overlaps another active profile; previous state restored";
      expect(localizeError("es", "APPLY_FAILED", detail)).toContain("Exclusión de carpeta de Defender:");
      expect(localizeError("es", "APPLY_FAILED", detail)).toContain("La carpeta se superpone con otra reserva de Defender.");
      expect(localizeError("es", "APPLY_FAILED", detail)).toContain("Se restauró el estado anterior.");
      expect(localizeError("en", "APPLY_FAILED", detail)).toContain("Defender folder exclusion:");
      expect(localizeError("en", "APPLY_FAILED", detail)).toContain("The folder overlaps another Defender reservation.");
    });

    it("shows conflicting folder names and paths in the selected language", () => {
      const warning = {
        step: "defender-path", message: "Defender exclusion overlaps another active profile", recovery_required: false,
        folder_conflict: { target: "C:\\Steam\\bin\\cef.win64", folders: ["C:\\Steam", "D:\\Games"], unidentified: false },
      };
      expect(applyWarningText("es", warning)).toBe("Se omitió la exclusión de carpeta de Defender.\nCarpeta: cef.win64 (C:\\Steam\\bin\\cef.win64)\nConflicto con: Steam (C:\\Steam)\nConflicto con: Games (D:\\Games)");
      expect(applyWarningText("en", warning)).toBe("Defender folder exclusion skipped.\nFolder: cef.win64 (C:\\Steam\\bin\\cef.win64)\nConflicts with: Steam (C:\\Steam)\nConflicts with: Games (D:\\Games)");
      warning.folder_conflict.folders = [];
      warning.folder_conflict.unidentified = true;
      expect(applyWarningText("es", warning)).toContain("No se pudieron identificar todas las carpetas en conflicto.");
      expect(applyWarningText("en", warning)).toContain("Some conflicting folders could not be identified.");
    });

    it("shows unknown causes and pending recovery without hiding the original diagnostic", () => {
      const detail = "cpu: Unknown failure; recovery required: RESTORE_FAILED: disk error";
      const result = localizeError("es", "APPLY_FAILED", detail);
      expect(result).toContain("CPU:");
      expect(result).toContain("aún requieren recuperación");
      expect(result).toContain("Detalles técnicos (idioma original): Unknown failure");
      expect(result).toContain("disk error");
    });

    it("reports restored global settings", () => {
      const result = localizeError("en", "SNAPSHOT_FAILED", "power: unexpected failure; previous settings restored");
      expect(result).toContain("The previous state was restored.");
      expect(result).toContain("Technical details (original language):");
    });

    it("shows an actionable launch error in the selected language", () => {
      expect(localizeError("es", "LAUNCH_FAILED", "Cannot start: Access denied (os error 5)")).toContain("Windows denegó el acceso");
      expect(localizeError("en", "LAUNCH_FAILED", "Cannot start: Access denied (os error 5)")).toContain("Windows denied access");
    });
  });

  it("identifies each Defender exclusion in both languages", () => {
    const warnings = [
      { step: "defender-path", message: "changed externally", retryable: false },
      { step: "defender-process", message: "changed externally", retryable: false },
    ];
    const spanish = skippedText("es", warnings).split("\n");
    expect(spanish[0]).toContain("Exclusión de carpeta de Defender:");
    expect(spanish[1]).toContain("Exclusión de proceso de Defender:");
    const english = skippedText("en", warnings).split("\n");
    expect(english[0]).toContain("Defender folder exclusion:");
    expect(english[1]).toContain("Defender process exclusion:");
  });
});

import { useEffect, useId, useRef, useState, type ReactNode } from 'react';

import { CloseIcon } from '@/components/ui/icons';
import { IconButton } from '@/components/ui/icon-button';

import { setSettings } from '@/shared/api/settings';
import {
  type TranslationScope,
  useTranslation,
} from '@/shared/hooks/use-translation';
import type {
  IBind,
  ILanguage,
  IPaper,
  ISettings,
} from '@/shared/interfaces/emulator';
import { cn } from '@/shared/styles/cn';
import { BUTTON, FIELD } from '@/shared/styles/patterns';
import { shortcutLabel } from '@/shared/utils/shortcut';
import { uiErrorKey } from '@/shared/utils/ui-error';
import { ZOOM_STEPS } from '@/shared/utils/zoom';

/** `ESC t` tables the emulator has, as in `codepage.rs`. */
const CODE_PAGES: { table: number; label: TranslationScope }[] = [
  { table: 0, label: 'settings.codePages.cp437' },
  { table: 2, label: 'settings.codePages.cp850' },
  { table: 3, label: 'settings.codePages.cp860' },
  { table: 4, label: 'settings.codePages.cp863' },
  { table: 5, label: 'settings.codePages.cp865' },
  { table: 16, label: 'settings.codePages.wpc1252' },
  { table: 17, label: 'settings.codePages.cp866' },
  { table: 18, label: 'settings.codePages.cp852' },
  { table: 19, label: 'settings.codePages.cp858' },
];

/** Each language in its own name, as language pickers do; `system` is translated. */
const LANGUAGES: { value: Exclude<ILanguage, 'system'>; name: string }[] = [
  { value: 'en', name: 'English' },
  { value: 'es', name: 'Español' },
  { value: 'pt-BR', name: 'Português (Brasil)' },
];

interface ISettingsPanelProps {
  open: boolean;
  settings: ISettings;
  onSaved: (settings: ISettings) => void;
  onClose: () => void;
}

/**
 * A panel that slides in over the receipts from the right; Esc or ✕ closes it. Every
 * change is saved and applied at once; the port waits for "Apply". While closed it is
 * `inert`, so focus and screen readers skip it.
 */
export function SettingsPanel({
  open,
  settings,
  onSaved,
  onClose,
}: ISettingsPanelProps) {
  const { t } = useTranslation();
  const closeButton = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    closeButton.current?.focus();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose();
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [open, onClose]);
  const [port, setPort] = useState(String(settings.port));
  const [error, setError] = useState<TranslationScope | null>(null);
  const portId = useId();
  const codePageId = useId();
  const languageId = useId();
  const zoomId = useId();

  const save = (next: ISettings) => {
    setError(null);
    setSettings(next)
      .then(onSaved)
      .catch((reason: unknown) => setError(uiErrorKey(reason)));
  };

  const portNumber = Number(port);
  const portValid =
    Number.isInteger(portNumber) && portNumber >= 1 && portNumber <= 65535;

  return (
    <aside
      id="settings-panel"
      aria-labelledby="settings-title"
      inert={!open}
      className={cn(
        'absolute inset-y-0 right-0 z-(--z-panel) flex w-[22rem] max-w-full flex-col border-l border-border bg-raised shadow-xl',
        'transition-[translate,visibility] duration-200 ease-out-quart motion-reduce:transition-none',
        open ? 'visible translate-x-0' : 'invisible translate-x-full',
      )}
    >
      <div className="flex items-center justify-between border-b border-border px-5 py-3">
        <h2 id="settings-title" className="text-base font-semibold text-ink">
          {t('settings.title')}
        </h2>
        <IconButton
          ref={closeButton}
          label={t('settings.close')}
          onClick={onClose}
        >
          <CloseIcon />
        </IconButton>
      </div>
      <div className="flex flex-1 flex-col gap-6 overflow-y-auto px-5 py-5">
        {error && (
          <p
            role="alert"
            className="rounded-md bg-error/10 px-3 py-2 text-sm text-ink"
          >
            {t(error)}
          </p>
        )}

        <Field label={t('settings.language.label')} htmlFor={languageId}>
          <select
            id={languageId}
            className={`${FIELD} w-full`}
            value={settings.language}
            onChange={(event) =>
              save({ ...settings, language: event.target.value as ILanguage })
            }
          >
            <option value="system">{t('settings.language.system')}</option>
            {LANGUAGES.map(({ value, name }) => (
              <option key={value} value={value} lang={value}>
                {name}
              </option>
            ))}
          </select>
        </Field>

        <Field label={t('settings.port.label')} htmlFor={portId}>
          <form
            className="flex gap-2"
            onSubmit={(event) => {
              event.preventDefault();
              if (portValid) save({ ...settings, port: portNumber });
            }}
          >
            <input
              id={portId}
              className={`${FIELD} w-28 tabular-nums`}
              inputMode="numeric"
              value={port}
              aria-invalid={!portValid}
              onChange={(event) =>
                setPort(event.target.value.replace(/\D/g, ''))
              }
            />
            <button
              type="submit"
              className={BUTTON}
              disabled={!portValid || portNumber === settings.port}
            >
              {t('settings.port.apply')}
            </button>
          </form>
          <Hint>
            {portValid ? t('settings.port.hint') : t('settings.errors.port')}
          </Hint>
        </Field>

        <Choice<IBind>
          label={t('settings.bind.label')}
          value={settings.bind}
          options={[
            { value: 'lan', label: t('settings.bind.lan') },
            { value: 'local', label: t('settings.bind.local') },
          ]}
          onChange={(bind) => save({ ...settings, bind })}
        />

        <Choice<IPaper>
          label={t('settings.paper.label')}
          hint={t('settings.paper.hint')}
          value={settings.paper}
          options={[
            { value: 'mm80', label: t('settings.paper.mm80') },
            { value: 'mm58', label: t('settings.paper.mm58') },
          ]}
          onChange={(paper) => save({ ...settings, paper })}
        />

        <Field label={t('settings.zoom.label')} htmlFor={zoomId}>
          <select
            id={zoomId}
            className={`${FIELD} w-full`}
            value={settings.zoom}
            onChange={(event) =>
              save({ ...settings, zoom: Number(event.target.value) })
            }
          >
            {ZOOM_STEPS.map((zoom) => (
              <option key={zoom} value={zoom}>
                {zoom}%
              </option>
            ))}
          </select>
          <Hint>
            {t('settings.zoom.hint', {
              in: shortcutLabel('+'),
              out: shortcutLabel('−'),
              reset: shortcutLabel('0'),
            })}
          </Hint>
        </Field>

        <Field label={t('settings.codePage.label')} htmlFor={codePageId}>
          <select
            id={codePageId}
            className={`${FIELD} w-full`}
            value={settings.code_page}
            onChange={(event) =>
              save({ ...settings, code_page: Number(event.target.value) })
            }
          >
            {CODE_PAGES.map(({ table, label }) => (
              <option key={table} value={table}>
                {t(label)}
              </option>
            ))}
          </select>
          <Hint>{t('settings.codePage.hint')}</Hint>
        </Field>

        <fieldset className="flex flex-col gap-2">
          <legend className="mb-2 text-sm font-semibold text-ink">
            {t('settings.sound.label')}
          </legend>
          <label className="flex items-center gap-2 text-sm text-ink">
            <input
              type="checkbox"
              checked={settings.sound}
              onChange={(event) =>
                save({ ...settings, sound: event.target.checked })
              }
            />
            {t('settings.sound.printing')}
          </label>
        </fieldset>
      </div>
    </aside>
  );
}

function Field({
  label,
  htmlFor,
  children,
}: {
  label: string;
  htmlFor: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-col gap-2">
      <label htmlFor={htmlFor} className="text-sm font-semibold text-ink">
        {label}
      </label>
      {children}
    </div>
  );
}

function Hint({ children }: { children: string }) {
  return <p className="text-xs text-muted">{children}</p>;
}

interface IChoiceProps<T extends string> {
  label: string;
  hint?: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
}

function Choice<T extends string>({
  label,
  hint,
  value,
  options,
  onChange,
}: IChoiceProps<T>) {
  const name = useId();
  return (
    <fieldset className="flex flex-col gap-2">
      <legend className="mb-2 text-sm font-semibold text-ink">{label}</legend>
      {options.map((option) => (
        <label
          key={option.value}
          className="flex items-center gap-2 text-sm text-ink"
        >
          <input
            type="radio"
            name={name}
            checked={value === option.value}
            onChange={() => onChange(option.value)}
          />
          {option.label}
        </label>
      ))}
      {hint && <Hint>{hint}</Hint>}
    </fieldset>
  );
}

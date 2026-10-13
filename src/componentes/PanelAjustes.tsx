// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef } from "react";
import { m } from "motion/react";
import type { Tema } from "../api/atlas";
import { normalizarError } from "../api/llamar";
import { elegirCarpetaMinecraft } from "../api/dialogo";
import { useAjustes, versionEfectiva, type EstadoAtlas } from "../estado/ajustes";
import { muelle } from "../estilos/movimiento";
import "./PanelAjustes.css";

const TEMAS: { valor: Tema; etiqueta: string }[] = [
  { valor: "claro", etiqueta: "Claro" },
  { valor: "oscuro", etiqueta: "Oscuro" },
  { valor: "sistema", etiqueta: "Sistema" },
];

/** Texto del estado del atlas para la versión elegida. */
function describirAtlas(atlas: EstadoAtlas, hayVersiones: boolean): string {
  if (!hayVersiones || atlas.tipo === "sinJar") return "Sin texturas: se usan colores planos.";
  switch (atlas.tipo) {
    case "listo":
      return `Texturas listas: Minecraft ${atlas.datos.version}, atlas de ${atlas.datos.ancho} × ${atlas.datos.alto} px.`;
    case "preparando":
      return "Preparando las texturas…";
    default:
      return "Comprobando las texturas…";
  }
}

/**
 * Ajustes de la app en un diálogo modal (Ctrl+, lo abre; Esc lo cierra):
 * tema, carpeta de Minecraft, versión de las texturas y fluidos en los
 * materiales. Cada cambio se guarda al momento. Usa <dialog> nativo, que ya
 * atrapa el foco e inhabilita el resto de la página.
 */
export function PanelAjustes() {
  const abierto = useAjustes((s) => s.abierto);
  const cerrar = useAjustes((s) => s.cerrar);
  const dialogo = useRef<HTMLDialogElement>(null);

  // El estado manda: el <dialog> se abre y se cierra para seguirlo.
  useEffect(() => {
    const d = dialogo.current;
    if (!d) return;
    if (abierto && !d.open) d.showModal();
    else if (!abierto && d.open) d.close();
  }, [abierto]);

  return (
    <dialog
      ref={dialogo}
      className="ajustes"
      aria-labelledby="titulo-ajustes"
      // Esc: se cierra por el estado, para que no se desfase del <dialog>.
      onCancel={(e) => {
        e.preventDefault();
        cerrar();
      }}
      onClick={(e) => {
        // Clic en el fondo (fuera de la caja).
        if (e.target === e.currentTarget) cerrar();
      }}
    >
      {/* Se monta en cada apertura: así entra siempre con su muelle. */}
      {abierto && (
        <m.div
          className="ajustes__caja"
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          transition={muelle("pulsacion")}
        >
          <header className="ajustes__cabecera">
            <h2 id="titulo-ajustes" className="ajustes__titulo rotulo">
              Ajustes
            </h2>
            <button type="button" className="boton-plano" onClick={cerrar} aria-keyshortcuts="Escape">
              Cerrar <kbd className="ajustes__tecla">Esc</kbd>
            </button>
          </header>

          <div className="ajustes__filas">
            <FilaTema />
            <FilaCarpeta />
            <FilaVersion />
            <FilaFluidos />
          </div>

          <Pie />
        </m.div>
      )}
    </dialog>
  );
}

/** Tema: tres botones de radio en una tira; la marca del elegido se desliza. */
function FilaTema() {
  const tema = useAjustes((s) => s.guardados.tema);
  const cambiar = useAjustes((s) => s.cambiar);
  return (
    <div className="ajustes__fila">
      <span id="ajustes-tema" className="ajustes__etiqueta">
        Tema
      </span>
      <div className="ajustes__control">
        <div className="segmentos" role="radiogroup" aria-labelledby="ajustes-tema">
          {TEMAS.map((t) => (
            <label key={t.valor} className="segmentos__opcion">
              <input
                type="radio"
                name="ajustes-tema"
                className="segmentos__entrada"
                value={t.valor}
                checked={tema === t.valor}
                onChange={() => void cambiar({ tema: t.valor })}
              />
              {tema === t.valor && (
                <m.span
                  layoutId="ajustes-tema-marca"
                  className="segmentos__marca"
                  transition={muelle("pulsacion")}
                  aria-hidden="true"
                />
              )}
              <span className="segmentos__texto">{t.etiqueta}</span>
            </label>
          ))}
        </div>
        <p className="ajustes__ayuda">«Sistema» sigue el modo claro u oscuro de Windows.</p>
      </div>
    </div>
  );
}

/** Carpeta `.minecraft`: la habitual o una elegida a mano. */
function FilaCarpeta() {
  const ruta = useAjustes((s) => s.guardados.rutaMinecraft ?? null);
  const porDefecto = useAjustes((s) => s.carpetaPorDefecto);
  const guardando = useAjustes((s) => s.guardando);
  const cambiar = useAjustes((s) => s.cambiar);

  const elegir = async () => {
    try {
      const nueva = await elegirCarpetaMinecraft(ruta ?? porDefecto ?? undefined);
      if (nueva) await cambiar({ rutaMinecraft: nueva });
    } catch (e) {
      useAjustes.setState({ error: normalizarError(e).mensaje });
    }
  };

  const mostrada = ruta ?? porDefecto;
  return (
    <div className="ajustes__fila" role="group" aria-labelledby="ajustes-carpeta">
      <span id="ajustes-carpeta" className="ajustes__etiqueta">
        Carpeta de Minecraft
      </span>
      <div className="ajustes__control">
        <p className="ajustes__ruta cifras" title={mostrada ?? undefined}>
          {mostrada ?? "No se encuentra .minecraft en las carpetas habituales."}
        </p>
        <p className="ajustes__ayuda">
          {ruta ? "Elegida a mano." : "La habitual (%APPDATA%\\.minecraft)."} Tiene que tener la
          carpeta «versions».
        </p>
        <div className="ajustes__botones">
          <button type="button" className="boton-plano" disabled={guardando} onClick={() => void elegir()}>
            Elegir carpeta…
          </button>
          {ruta && (
            <button
              type="button"
              className="boton-plano"
              disabled={guardando}
              onClick={() => void cambiar({ rutaMinecraft: undefined })}
            >
              Usar la habitual
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

/** Versión de Minecraft de la que salen las texturas (y los nombres). */
function FilaVersion() {
  const versiones = useAjustes((s) => s.versiones);
  const cargando = useAjustes((s) => s.cargando);
  const guardando = useAjustes((s) => s.guardando);
  const guardada = useAjustes((s) => s.guardados.versionMinecraft);
  const atlas = useAjustes((s) => s.atlas);
  const cambiar = useAjustes((s) => s.cambiar);

  // Releases primero (el orden ya viene así de Rust) y las demás detrás.
  const releases = versiones.filter((v) => v.esRelease);
  const otras = versiones.filter((v) => !v.esRelease);
  const sinVersiones = !cargando && versiones.length === 0;
  const automatica = versionEfectiva(versiones, null);
  const noInstalada = guardada !== null && !cargando && !versiones.some((v) => v.id === guardada);

  return (
    <div className="ajustes__fila">
      <label htmlFor="ajustes-version" className="ajustes__etiqueta">
        Versión de las texturas
      </label>
      <div className="ajustes__control">
        <select
          id="ajustes-version"
          className="ajustes__selector cifras"
          value={guardada ?? ""}
          disabled={cargando || sinVersiones || guardando}
          onChange={(e) => void cambiar({ versionMinecraft: e.target.value || null })}
        >
          <option value="">
            {cargando
              ? "Buscando versiones…"
              : sinVersiones
                ? "Ninguna versión instalada"
                : `Automática (${automatica})`}
          </option>
          {noInstalada && (
            <option value={guardada}>{guardada} (no está en esta carpeta: se usa la automática)</option>
          )}
          {releases.length > 0 && (
            <optgroup label="Versiones oficiales">
              {releases.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.id}
                </option>
              ))}
            </optgroup>
          )}
          {otras.length > 0 && (
            <optgroup label="Otras (snapshots, Fabric, Forge…)">
              {otras.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.id} (no es release)
                </option>
              ))}
            </optgroup>
          )}
        </select>
        <p className="ajustes__estado" role="status" aria-live="polite">
          {describirAtlas(atlas, versiones.length > 0)}
        </p>
        <p className="ajustes__ayuda">
          Las texturas y los nombres de los ítems se leen del jar de tu Minecraft; la app no los
          redistribuye.
        </p>
      </div>
    </div>
  );
}

/** Fluidos en la lista de materiales: interruptor. */
function FilaFluidos() {
  const incluir = useAjustes((s) => s.guardados.incluirFluidos);
  const cambiar = useAjustes((s) => s.cambiar);
  return (
    <div className="ajustes__fila">
      <span className="ajustes__etiqueta">Materiales</span>
      <div className="ajustes__control">
        <label className="interruptor">
          <input
            type="checkbox"
            role="switch"
            className="interruptor__entrada"
            checked={incluir}
            aria-describedby="ajustes-fluidos-ayuda"
            onChange={(e) => void cambiar({ incluirFluidos: e.target.checked })}
          />
          <span className="interruptor__pista" aria-hidden="true">
            <m.span
              className="interruptor__boton"
              initial={false}
              animate={{ x: incluir ? 14 : 0 }}
              transition={muelle("pulsacion")}
            />
          </span>
          <span>Contar agua y lava</span>
        </label>
        <p id="ajustes-fluidos-ayuda" className="ajustes__ayuda">
          Solo las fuentes, como cubos de agua o de lava.
        </p>
      </div>
    </div>
  );
}

/** Errores y estado del guardado, y los atajos. */
function Pie() {
  const error = useAjustes((s) => s.error);
  const guardando = useAjustes((s) => s.guardando);
  return (
    <footer className="ajustes__pie">
      {error ? (
        <p className="ajustes__error" role="alert">
          {error}
        </p>
      ) : (
        <p className="ajustes__guardado" aria-live="polite">
          {guardando ? "Guardando…" : "Los cambios se guardan al momento."}
        </p>
      )}
      <p className="ajustes__atajos cifras" aria-hidden="true">
        <kbd className="ajustes__tecla">Ctrl+,</kbd> abre · <kbd className="ajustes__tecla">Esc</kbd>{" "}
        cierra
      </p>
    </footer>
  );
}

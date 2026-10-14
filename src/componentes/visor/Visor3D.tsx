// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { lazy, Suspense, useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { m } from "motion/react";
import { obtenerAtlas, type AtlasCargado } from "../../api/atlas";
import { cargarMalla, type Malla } from "../../api/visor";
import { useAjustes } from "../../estado/ajustes";
import { useEdicion } from "../../estado/edicion";
import { useMateriales } from "../../estado/materiales";
import { useTexturas } from "../../estado/texturas";
import { useVisor } from "../../estado/visor";
import { datosDeFoco, espacioBloqueadoPor } from "../../logica/atajos";
import { dormir } from "../../logica/tiempo";
import { muelle } from "../../estilos/movimiento";
import { BotonExportar, ConmutadorModo } from "../edicion/BarraEdicion";
import { DeslizadorCapas } from "./DeslizadorCapas";
import { PanelBloque } from "./PanelBloque";
import "./Visor3D.css";

// Three.js va en un trozo aparte: se pide al abrir el schematic, a la vez
// que la malla, y no pesa en el arranque de la biblioteca.
const cargarLienzo = () => import("./Lienzo");
const Lienzo = lazy(cargarLienzo);

type Carga =
  | { tipo: "cargando" }
  | { tipo: "error"; mensaje: string }
  | { tipo: "lista"; malla: Malla; atlas: AtlasCargado | null };

/** Esperas (ms) entre reintentos si el atlas no se pudo preparar todavía. */
const ESPERAS_ATLAS = [1000, 2000, 4000];

const mensajeDe = (e: unknown) =>
  typeof e === "object" && e !== null && "mensaje" in e && typeof e.mensaje === "string"
    ? e.mensaje
    : e instanceof Error
      ? e.message
      : String(e);

/** Entradas que no son de texto: con el foco en ellas los atajos siguen valiendo. */
const ENTRADAS_SIN_TEXTO = new Set(["radio", "checkbox", "button", "range"]);

/** ¿El evento de teclado viene de un campo de texto? Entonces no es un atajo. */
const escribiendo = (e: KeyboardEvent) =>
  e.target instanceof HTMLElement &&
  (e.target.isContentEditable ||
    /^(TEXTAREA|SELECT)$/.test(e.target.tagName) ||
    (e.target instanceof HTMLInputElement && !ENTRADAS_SIN_TEXTO.has(e.target.type)));

interface Props {
  id: string;
  /** Lo que se ve mientras llega la malla (la miniatura plana); luego se funde. */
  marcador: ReactNode;
}

/**
 * Visor 3D de un schematic. Vive dentro de `.schematic__visor`, el elemento
 * compartido con la lámina de la biblioteca: la miniatura sigue ahí como
 * marcador hasta que la malla se ha dibujado una vez, y entonces el lienzo
 * aparece por encima con un fundido.
 */
export function Visor3D({ id, marcador }: Props) {
  const [carga, setCarga] = useState<Carga>({ tipo: "cargando" });
  const [lista, setLista] = useState(false);
  // Al cambiar de texturas (Ajustes) se vuelven a pedir la malla y el atlas
  // del mismo schematic, sin pasar por «cargando» ni reiniciar las capas.
  const generacionTexturas = useTexturas((s) => s.generacion);
  // Tras aplicar o restaurar una edición, lo mismo: la copia cambió en disco.
  const revision = useEdicion((s) => s.revision);
  const modo = useEdicion((s) => s.modo);
  const idCargado = useRef<string | null>(null);

  useEffect(() => {
    let activo = true;
    const recarga = idCargado.current === id;
    if (!recarga) {
      setCarga({ tipo: "cargando" });
      setLista(false);
      cargarLienzo().catch(() => {});
    }

    /** Atlas que casa con la malla, o null (sin jar o sin textura que mostrar). */
    const atlasDe = async (malla: Malla) => {
      if (!malla.atlas) return null;
      try {
        return await obtenerAtlas(malla.atlas.version);
      } catch (e) {
        console.warn("No se pudo cargar el atlas de texturas:", e);
        return null;
      }
    };

    (async () => {
      const malla = await cargarMalla(id);
      const atlas = await atlasDe(malla);
      if (!activo) return;
      // Una edición no debería cambiar el alto, pero si cambia, las capas
      // de antes no valen.
      if (!recarga || malla.tamano[1] !== useVisor.getState().alto)
        useVisor.getState().reiniciar(malla.tamano[1]);
      idCargado.current = id;
      setCarga({ tipo: "lista", malla, atlas });
      if (malla.atlas && atlas) return;

      // La malla salió sin texturas (el atlas aún no estaba preparado): se
      // espera al atlas y, cuando está, se pide la malla otra vez, porque los
      // índices de textura de los materiales solo se resuelven al mallar.
      for (let intento = 0; activo && intento <= ESPERAS_ATLAS.length; intento++) {
        try {
          const a = await obtenerAtlas(); // espera a que se genere
          if (!a) return; // sin jar: colores planos
          const conTexturas = await cargarMalla(id);
          if (!activo) return;
          if (conTexturas.atlas) {
            const atlasFinal = await atlasDe(conTexturas);
            if (activo && atlasFinal) setCarga({ tipo: "lista", malla: conTexturas, atlas: atlasFinal });
            return;
          }
        } catch (e) {
          console.warn("Atlas aún no disponible:", e);
        }
        if (intento < ESPERAS_ATLAS.length) await dormir(ESPERAS_ATLAS[intento]);
      }
    })().catch((e) => activo && setCarga({ tipo: "error", mensaje: mensajeDe(e) }));
    return () => {
      activo = false;
    };
  }, [id, generacionTexturas, revision]);

  // Atajos: F encuadra, M cambia entre Visualizar y Modificar, Esc cierra la
  // lista de parecidos o la selección (antes de que App vuelva a la
  // biblioteca: por eso en captura y con preventDefault), P el panel de
  // rendimiento en desarrollo.
  const hayMalla = carga.tipo === "lista";
  // Espacio: giro automático. Estado local: al salir del visor se pierde y al
  // volver a abrir empieza apagado.
  const [girando, setGirando] = useState(false);
  useEffect(() => {
    if (!hayMalla) return;
    const alPulsar = (e: KeyboardEvent) => {
      if (e.defaultPrevented || e.ctrlKey || e.metaKey || e.altKey || escribiendo(e)) return;
      if (useAjustes.getState().abierto) return; // el diálogo de ajustes tiene el teclado
      const edicion = useEdicion.getState();
      if (edicion.confirmacion) return; // y el de confirmar la edición también
      const s = useVisor.getState();
      // Espacio solo si el foco no está en algo que lo use (botón, campo, diálogo).
      if (e.key === " " || e.code === "Space") {
        if (e.repeat || e.shiftKey || espacioBloqueadoPor(datosDeFoco(document.activeElement))) return;
        e.preventDefault();
        setGirando((g) => !g);
        return;
      }
      // Esc cierra de dentro afuera: la lista de un destino de la familia, la
      // familia, la lista del bloque y, por último, la selección.
      if (e.key === "Escape" && edicion.lista?.objetivo.tipo === "reemplazo") {
        e.preventDefault();
        edicion.cerrarLista();
      } else if (e.key === "Escape" && edicion.familia) {
        e.preventDefault();
        edicion.cerrarFamilia();
      } else if (e.key === "Escape" && edicion.lista) {
        e.preventDefault();
        edicion.cerrarLista();
      } else if (e.key === "Escape" && s.seleccion) {
        e.preventDefault();
        s.limpiarSeleccion();
      } else if (e.key === "m" || e.key === "M") {
        e.preventDefault();
        edicion.alternarModo();
      } else if (e.key === "f" || e.key === "F") {
        e.preventDefault();
        s.pedirEncuadre();
      } else if (import.meta.env.DEV && (e.key === "p" || e.key === "P")) {
        s.alternarRendimiento();
      }
    };
    window.addEventListener("keydown", alPulsar, true);
    return () => window.removeEventListener("keydown", alPulsar, true);
  }, [hayMalla]);

  // La pantalla de materiales saca de la malla los iconos y la Y del origen.
  useEffect(() => {
    if (carga.tipo === "lista") useMateriales.getState().publicarModelo(id, carga.malla, carga.atlas);
  }, [carga, id]);

  const alListo = useCallback(() => setLista(true), []);
  const vacia = carga.tipo === "lista" && carga.malla.chunks.length === 0;
  const transicion = muelle("vista");

  let nota: ReactNode;
  if (carga.tipo === "cargando") nota = <p className="visor3d__nota">Cargando el modelo…</p>;
  else if (carga.tipo === "error")
    nota = (
      <p className="visor3d__nota visor3d__nota--error" role="alert">
        No se pudo cargar el modelo en 3D: {carga.mensaje}
      </p>
    );
  else if (vacia) nota = <p className="visor3d__nota">Este schematic no tiene bloques visibles.</p>;
  else
    nota = (
      <p className="visor3d__nota">
        Arrastrar: girar · Botón derecho: desplazar · Rueda: acercar · Con el foco en el visor: flechas giran, +/− acercan · Espacio: girar ·{" "}
        {modo === "modificar"
          ? "Clic: elegir bloque · Mayús+clic: su familia"
          : "Clic: inspeccionar"}
      </p>
    );

  return (
    <>
      <m.div
        className="visor3d__marcador"
        initial={false}
        animate={{ opacity: lista && !vacia ? 0 : 1 }}
        transition={transicion}
      >
        {marcador}
      </m.div>

      {carga.tipo === "lista" && !vacia && (
        <m.div
          className={modo === "modificar" ? "visor3d visor3d--modificar" : "visor3d"}
          initial={{ opacity: 0 }}
          animate={{ opacity: lista ? 1 : 0 }}
          transition={transicion}
          inert={!lista}
        >
          <div className="visor3d__lienzo">
            <Suspense fallback={null}>
              <Lienzo
                id={id}
                malla={carga.malla}
                atlas={carga.atlas}
                girando={girando}
                onListo={alListo}
              />
            </Suspense>
          </div>

          <PanelBloque />

          <div className="visor3d__herramientas">
            <ConmutadorModo />
            {modo === "modificar" && <BotonExportar id={id} />}
            <button
              type="button"
              className="boton-plano"
              onClick={() => useVisor.getState().pedirEncuadre()}
            >
              Encuadrar
              <kbd>F</kbd>
            </button>
          </div>

          <DeslizadorCapas origenY={carga.malla.origen[1]} />
        </m.div>
      )}

      {nota}
    </>
  );
}

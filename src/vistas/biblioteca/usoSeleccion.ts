// Schematic Lab — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.
// Elemento activo de la colección de la biblioteca: tabulación, foco y teclado.

import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as EventoTecladoReact,
  type RefObject,
} from "react";
import type { Entrada } from "../../api/biblioteca";
import { useBiblioteca } from "../../estado/biblioteca";
import { useFoco } from "../../estado/foco";
import { esEditable } from "./usoAtajosBiblioteca";

interface OpcionesSeleccion {
  /** Cuerpo desplazable que contiene la colección. */
  cuerpo: RefObject<HTMLDivElement | null>;
  /** La rejilla o la tabla (para contar columnas). */
  contenedor: RefObject<HTMLElement | null>;
  visibles: Entrada[];
  /** Elemento activo al montar (el schematic del que se vuelve), o `null`. */
  activoInicial: string | null;
  /** F2 sobre el elemento activo. */
  alRenombrar: (id: string) => void;
}

/**
 * Tabindex itinerante: un solo elemento de la colección está en la
 * tabulación; si el activo deja de verse, lo es el primero. Lleva también el
 * foco (peticiones de fuera, vecino al eliminar) y el teclado de la colección.
 * Las funciones devueltas son estables.
 */
export function usoSeleccion({
  cuerpo,
  contenedor,
  visibles,
  activoInicial,
  alRenombrar,
}: OpcionesSeleccion) {
  const modoVista = useBiblioteca((s) => s.modoVista);
  const pedido = useFoco((s) => s.pedido);
  const [activoId, setActivoId] = useState<string | null>(activoInicial);

  const activo = useMemo(
    () => (visibles.some((e) => e.id === activoId) ? activoId : (visibles[0]?.id ?? null)),
    [visibles, activoId],
  );

  // Tabindex itinerante aplicado en el DOM: cambiar de elemento activo no
  // vuelve a pintar ninguna lámina (cada repintado de un nodo con layout
  // obliga a Motion a medir las 500).
  useLayoutEffect(() => {
    const raiz = cuerpo.current;
    if (!raiz) return;
    for (const b of raiz.querySelectorAll<HTMLElement>('[data-abrir][tabindex="0"]')) {
      if (b.dataset.abrir !== activo) b.tabIndex = -1;
    }
    if (activo) {
      const b = raiz.querySelector<HTMLElement>(`[data-abrir="${CSS.escape(activo)}"]`);
      if (b) b.tabIndex = 0;
    }
  });

  // Lo último de cada render, para las funciones estables.
  const ultimo = useRef({ visibles, activo });
  useLayoutEffect(() => {
    ultimo.current = { visibles, activo };
  });

  const botonDe = useCallback(
    (id: string) =>
      cuerpo.current?.querySelector<HTMLElement>(`[data-abrir="${CSS.escape(id)}"]`) ?? null,
    [cuerpo],
  );

  const enfocar = useCallback(
    (id: string) => {
      setActivoId(id);
      botonDe(id)?.focus();
    },
    [botonDe],
  );

  /** Lleva el foco al elemento activo; `false` si no hay ninguno. */
  const enfocarActivo = useCallback(() => {
    const { activo: id } = ultimo.current;
    if (!id) return false;
    enfocar(id);
    return true;
  }, [enfocar]);

  const eliminar = useCallback(
    (id: string) => {
      const lista = ultimo.current.visibles;
      const i = lista.findIndex((e) => e.id === id);
      const siguiente = lista[i + 1] ?? lista[i - 1];
      // El foco pasa al vecino antes de que la lámina desaparezca; si
      // estaba en otra parte (borrado con el ratón), no se toca.
      const conFoco = cuerpo.current?.contains(document.activeElement) ?? false;
      if (siguiente) {
        setActivoId(siguiente.id);
        if (conFoco) botonDe(siguiente.id)?.focus();
      }
      useBiblioteca.getState().eliminar(id);
    },
    [cuerpo, botonDe],
  );

  // Peticiones de foco de fuera (Deshacer del aviso de borrado).
  useEffect(() => {
    if (!pedido) return;
    if (visibles.some((e) => e.id === pedido.id)) enfocar(pedido.id);
    useFoco.getState().atender();
  }, [pedido, visibles, enfocar]);

  // ── Teclado de la colección ──
  const columnas = (): number => {
    if (modoVista === "lista" || !contenedor.current) return 1;
    const pistas = getComputedStyle(contenedor.current).gridTemplateColumns.split(" ");
    return Math.max(1, pistas.filter(Boolean).length);
  };

  const alPulsarTecla = (e: EventoTecladoReact<HTMLDivElement>) => {
    if (esEditable(e.target) || e.altKey || e.ctrlKey || e.metaKey) return;
    const lista = visibles;
    const i = lista.findIndex((x) => x.id === activo);
    if (i < 0) return;
    const cols = columnas();
    const ultimoIndice = lista.length - 1;
    const alturaFila = modoVista === "lista" ? 40 : 240;
    const filasPorPagina = Math.max(
      1,
      Math.floor((cuerpo.current?.clientHeight ?? alturaFila) / alturaFila) - 1,
    );

    let destino: number | null = null;
    switch (e.key) {
      case "ArrowRight":
        destino = modoVista === "rejilla" ? Math.min(i + 1, ultimoIndice) : null;
        break;
      case "ArrowLeft":
        destino = modoVista === "rejilla" ? Math.max(i - 1, 0) : null;
        break;
      case "ArrowDown":
        // En la última fila incompleta, bajar lleva al último elemento.
        destino =
          i + cols <= ultimoIndice
            ? i + cols
            : Math.floor(i / cols) < Math.floor(ultimoIndice / cols)
              ? ultimoIndice
              : i;
        break;
      case "ArrowUp":
        destino = i - cols >= 0 ? i - cols : i;
        break;
      case "PageDown":
        destino = Math.min(i + cols * filasPorPagina, ultimoIndice);
        break;
      case "PageUp":
        destino = Math.max(i - cols * filasPorPagina, 0);
        break;
      case "Home":
        destino = 0;
        break;
      case "End":
        destino = ultimoIndice;
        break;
      case "F2":
        e.preventDefault();
        alRenombrar(lista[i].id);
        return;
      case "Delete":
        e.preventDefault();
        eliminar(lista[i].id);
        return;
      default:
        return;
    }
    if (destino === null) return;
    e.preventDefault();
    enfocar(lista[destino].id);
  };

  return {
    /** El elemento ha recibido el foco: pasa a ser el activo. */
    activar: setActivoId,
    botonDe,
    enfocar,
    enfocarActivo,
    /** Elimina pasando antes el foco (y el activo) al vecino. */
    eliminar,
    alPulsarTecla,
  };
}

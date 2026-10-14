// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import {
  lazy,
  Suspense,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ComponentRef,
} from "react";
import { useFrame, useThree } from "@react-three/fiber";
import { useReducedMotion } from "motion/react";
import { GizmoHelper, GizmoViewport, Grid, OrbitControls } from "@react-three/drei";
import {
  BoxGeometry,
  Color,
  DoubleSide,
  EdgesGeometry,
  LineBasicMaterial,
  PerspectiveCamera,
  Raycaster,
  Spherical,
  Vector2,
  Vector3,
} from "three";
import { bloqueEn, cargarMalla, type Malla } from "../../api/visor";
import { useEdicion } from "../../estado/edicion";
import { capaMax, capaMin, useVisor } from "../../estado/visor";
import { colorToken, hexToken, usoTema } from "./colores";
import { aplicarCorte, crearGrupo, lanzarRayo, type GrupoVisor } from "./mallaThree";
import { montarCambios } from "./previaEdicion";
import { aplicarAtlas, crearUniformsVisor, type AtlasVisor, type UniformsVisor } from "./sombreado";
import { usoPreviaEdicion } from "./usoPreviaEdicion";

/** Espera tras el último cambio del corte antes de pedir la malla cerrada. */
const ESPERA_REMALLADO = 250;
/** Más movimiento que esto entre pulsar y soltar es orbitar, no hacer clic. */
const TOLERANCIA_CLIC = 4;
/** Margen del gizmo de ejes desde la esquina inferior derecha (px). */
const MARGEN_GIZMO: [number, number] = [64, 64];
/** Velocidad del giro automático (OrbitControls: 2 = una vuelta cada 30 s). Con «reducir movimiento», un tercio. */
const VELOCIDAD_GIRO = 1.5;
/** Medio lado del cuadrado que ocupa el gizmo en pantalla (px). */
const RADIO_GIZMO = 60;

// r3f-perf solo existe en desarrollo: en producción `import.meta.env.DEV` es
// false, la rama entera desaparece y el import dinámico no llega al bundle.
const PanelRendimiento = import.meta.env.DEV
  ? lazy(() => import("r3f-perf").then((m) => ({ default: m.Perf })))
  : null;

type Controles = ComponentRef<typeof OrbitControls>;

interface Props {
  id: string;
  malla: Malla;
  /** Texturas del schematic, o `null` (colores planos). */
  atlas: AtlasVisor | null;
  /** Giro automático lento (tecla Espacio). */
  girando: boolean;
  /** Se llama tras el primer fotograma con la malla dibujada. */
  onListo: () => void;
}

/** Coloca la cámara para que el modelo entero quepa en pantalla. */
function encuadrar(camara: PerspectiveCamera, controles: Controles | null, malla: Malla) {
  const [ancho, alto, largo] = malla.tamano;
  const centro = new Vector3(ancho / 2, alto / 2, largo / 2);
  const radio = Math.max(Math.hypot(ancho, alto, largo) / 2, 1);
  const vertical = (camara.fov * Math.PI) / 180;
  const horizontal = 2 * Math.atan(Math.tan(vertical / 2) * camara.aspect);
  const distancia = (radio / Math.sin(Math.min(vertical, horizontal) / 2)) * 1.05;
  // Desde +X +Z y algo por encima, como la miniatura isométrica.
  const direccion = new Vector3(1, 0.85, 1.25).normalize();
  camara.position.copy(centro).addScaledVector(direccion, distancia);
  camara.near = Math.max(distancia / 1000, 0.05);
  camara.far = distancia + radio * 8;
  camara.updateProjectionMatrix();
  if (controles) {
    controles.target.copy(centro);
    controles.maxDistance = distancia * 6;
    controles.update();
  } else {
    camara.lookAt(centro);
  }
}

export function Escena({ id, malla, atlas, girando, onListo }: Props) {
  const camera = useThree((s) => s.camera) as PerspectiveCamera;
  const gl = useThree((s) => s.gl);
  const invalidate = useThree((s) => s.invalidate);
  const controles = useRef<Controles>(null);
  const tema = usoTema();
  // Sin inercia al girar con «reducir movimiento» del sistema.
  const reducir = useReducedMotion();
  // El lienzo dibuja bajo demanda: el giro automático se mantiene solo (cada
  // update de los controles emite «change» e invalida), pero hay que arrancarlo.
  useEffect(() => {
    invalidate();
  }, [girando, invalidate]);
  // El «change» de los controles no basta para sostener el giro: medido en el
  // exe de release, a veces el bucle se paraba tras 1-4 fotogramas al arrancar
  // (con un bloque pinchado en Modificar). Pedir otro fotograma desde dentro de
  // uno lo mantiene vivo mientras gira.
  useFrame(() => {
    if (girando) invalidate();
  });

  // ── Mallas: la completa siempre; la cerrada (remallada con el corte) si la hay.
  // Uniforms comunes a todas las mallas de la escena: corte y marcas de la edición.
  const corte = useMemo<UniformsVisor>(crearUniformsVisor, []);
  // El atlas va por ref: al llegar solo cambian los uniforms y las tablas
  // (efecto de más abajo), no se rehace la geometría.
  const atlasRef = useRef(atlas);
  atlasRef.current = atlas;
  const completa = useMemo(
    () => crearGrupo(malla, corte, atlasRef.current),
    [malla, corte],
  );
  useEffect(() => () => completa.dispose(), [completa]);
  const [cerrada, setCerrada] = useState<{ desde: number; hasta: number; g: GrupoVisor } | null>(
    null,
  );
  useEffect(() => () => cerrada?.g.dispose(), [cerrada]);
  const cerradaRef = useRef(cerrada);
  cerradaRef.current = cerrada;
  const visible = useRef<GrupoVisor>(completa);

  /** Aplica el corte del estado: uniforms, chunks visibles y qué malla se ve. */
  const aplicar = useRef(() => {});
  aplicar.current = () => {
    const s = useVisor.getState();
    const desde = capaMin(s);
    const hasta = capaMax(s);
    corte.uDesde.value = desde;
    corte.uHasta.value = hasta;
    const c = cerradaRef.current;
    const usarCerrada = c !== null && c.desde === desde && c.hasta === hasta;
    completa.grupo.visible = !usarCerrada;
    if (c) c.g.grupo.visible = usarCerrada;
    visible.current = usarCerrada && c ? c.g : completa;
    aplicarCorte(visible.current, desde, hasta);
    invalidate();
  };

  // Remallado cerrado: al soltar el deslizador (o tras la última tecla).
  const peticion = useRef(0);
  useEffect(() => {
    let espera: number | undefined;
    const programar = () => {
      window.clearTimeout(espera);
      const s = useVisor.getState();
      const desde = capaMin(s);
      const hasta = capaMax(s);
      const entero = desde <= 0 && hasta >= s.alto - 1;
      if (entero) {
        peticion.current++;
        if (cerradaRef.current) setCerrada(null);
        return;
      }
      const c = cerradaRef.current;
      if (s.arrastrando || (c && c.desde === desde && c.hasta === hasta)) return;
      espera = window.setTimeout(() => {
        const n = ++peticion.current;
        cargarMalla(id, [desde, hasta])
          .then((m) => {
            if (n !== peticion.current) return;
            setCerrada({ desde, hasta, g: crearGrupo(m, corte, atlasRef.current) });
          })
          // Sin malla cerrada el corte sigue siendo correcto, solo hueco.
          .catch((e) => console.warn("No se pudo remallar el corte:", e));
      }, ESPERA_REMALLADO);
    };
    // Malla nueva (p. ej. la versión con texturas): la cerrada era de la
    // anterior, se descarta y, si hay corte, se vuelve a pedir.
    if (cerradaRef.current) {
      cerradaRef.current = null;
      setCerrada(null);
    }
    aplicar.current();
    programar();
    const quitar = useVisor.subscribe((s, antes) => {
      const cambioRango = capaMin(s) !== capaMin(antes) || capaMax(s) !== capaMax(antes);
      if (cambioRango) aplicar.current();
      if (cambioRango || s.arrastrando !== antes.arrastrando) programar();
    });
    return () => {
      quitar();
      window.clearTimeout(espera);
      peticion.current++;
    };
  }, [id, corte, completa]);

  // Al llegar (o irse) la malla cerrada, se cambia antes de pintar: sin parpadeo.
  useLayoutEffect(() => aplicar.current(), [cerrada, completa]);

  // ── Edición: previsualización de los pendientes y resaltados.
  const { previa, cambios: cambiosPrevia } = usoPreviaEdicion(malla, atlas !== null);

  // Atlas nuevo con la misma malla (p. ej. llegó después de pintar) o
  // pendientes nuevos: se reescriben las tablas de los materiales ya creados
  // (las dos mallas tienen su propia tabla). Nunca se toca la geometría.
  useLayoutEffect(() => {
    aplicarAtlas(completa.materiales, malla.materiales, atlas, previa);
    if (cerrada) aplicarAtlas(cerrada.g.materiales, cerrada.g.datos, atlas, previa);
    invalidate();
  }, [atlas, completa, malla, cerrada, previa, invalidate]);

  // ── Encuadre: al cargar y cada vez que se pide (botón o tecla F).
  const encuadre = useVisor((s) => s.encuadre);
  // No depende de `malla`: al llegar la versión con texturas (mismo modelo,
  // otra malla) la cámara se queda donde la dejó el usuario.
  const mallaRef = useRef(malla);
  mallaRef.current = malla;
  useLayoutEffect(() => {
    encuadrar(camera, controles.current, mallaRef.current);
    invalidate();
  }, [camera, id, encuadre, invalidate]);

  // ── Picking: clic sin arrastre → bloque bajo el puntero.
  useEffect(() => {
    const lienzo = gl.domElement;
    const raycaster = new Raycaster();
    const ndc = new Vector2();
    let inicio: { x: number; y: number } | null = null;
    const alPulsar = (e: PointerEvent) => {
      inicio = e.button === 0 ? { x: e.clientX, y: e.clientY } : null;
    };
    const alSoltar = (e: PointerEvent) => {
      if (!inicio || e.button !== 0) return;
      const movido = Math.hypot(e.clientX - inicio.x, e.clientY - inicio.y);
      inicio = null;
      if (movido >= TOLERANCIA_CLIC) return;
      const r = lienzo.getBoundingClientRect();
      // El gizmo de ejes tiene sus propios clics.
      const gx = r.right - MARGEN_GIZMO[0];
      const gy = r.bottom - MARGEN_GIZMO[1];
      if (Math.abs(e.clientX - gx) < RADIO_GIZMO && Math.abs(e.clientY - gy) < RADIO_GIZMO) return;
      ndc.set(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
      raycaster.setFromCamera(ndc, camera);
      const s = useVisor.getState();
      const impacto = lanzarRayo(visible.current, raycaster, capaMin(s), capaMax(s));
      if (!impacto) {
        s.limpiarSeleccion();
        return;
      }
      const { celda } = impacto;
      // Mayús+clic en Modificar: además, seleccionar toda su familia.
      const familia = e.shiftKey && useEdicion.getState().modo === "modificar";
      s.seleccionar(celda);
      bloqueEn(id, celda[0], celda[1], celda[2])
        .then((b) => {
          useVisor.getState().completarSeleccion(celda, b);
          // Solo si la selección sigue siendo esta (no llegó otro clic antes).
          const sel = useVisor.getState().seleccion;
          if (familia && b && sel?.bloque === b) useEdicion.getState().seleccionarFamilia(b.estado);
        })
        .catch(() => useVisor.getState().completarSeleccion(celda, null));
    };
    lienzo.addEventListener("pointerdown", alPulsar);
    lienzo.addEventListener("pointerup", alSoltar);
    return () => {
      lienzo.removeEventListener("pointerdown", alPulsar);
      lienzo.removeEventListener("pointerup", alSoltar);
    };
  }, [gl, camera, id]);

  // ── Teclado: el lienzo recibe foco (Tab). Con el foco en él, las flechas
  // giran la cámara alrededor del objetivo (izquierda/derecha: azimut;
  // arriba/abajo: elevación, dentro de los límites de OrbitControls) y +/-
  // acercan y alejan. Mayús da pasos más grandes. Cada pulsación es un paso
  // discreto (sin animación), así que no hay nada que reducir con
  // prefers-reduced-motion.
  useEffect(() => {
    const lienzo = gl.domElement;
    const ctrl = controles.current;
    lienzo.tabIndex = 0;
    lienzo.setAttribute("role", "application");
    lienzo.setAttribute(
      "aria-label",
      "Visor 3D del schematic. Flechas: girar la vista. Más y menos: acercar y alejar. F: encuadrar",
    );
    if (!ctrl) return;
    const alPulsar = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey || e.altKey) return;
      const paso = e.shiftKey ? 0.25 : 0.08; // radianes por pulsación
      const factor = e.shiftKey ? 0.75 : 0.9; // escala de la distancia al acercar
      let azimut = 0;
      let elevacion = 0;
      let escala = 1;
      switch (e.key) {
        case "ArrowLeft": azimut = -paso; break;
        case "ArrowRight": azimut = paso; break;
        case "ArrowUp": elevacion = -paso; break; // cámara hacia arriba (polar menor)
        case "ArrowDown": elevacion = paso; break;
        case "+": case "=": escala = factor; break;
        case "-": case "_": escala = 1 / factor; break;
        default:
          if (e.code === "NumpadAdd") escala = factor;
          else if (e.code === "NumpadSubtract") escala = 1 / factor;
          else return;
      }
      e.preventDefault();
      const esfera = new Spherical().setFromVector3(camera.position.clone().sub(ctrl.target));
      esfera.theta += azimut;
      esfera.phi = Math.min(ctrl.maxPolarAngle, Math.max(ctrl.minPolarAngle, esfera.phi + elevacion));
      esfera.radius = Math.min(ctrl.maxDistance, Math.max(ctrl.minDistance, esfera.radius * escala));
      esfera.makeSafe();
      camera.position.copy(ctrl.target).add(new Vector3().setFromSpherical(esfera));
      ctrl.update();
      invalidate();
    };
    lienzo.addEventListener("keydown", alPulsar);
    return () => lienzo.removeEventListener("keydown", alPulsar);
  }, [gl, camera, invalidate]);

  // ── Aviso de listo tras el primer fotograma con la malla.
  const avisado = useRef(false);
  const cuadroAviso = useRef(0);
  useFrame(() => {
    if (avisado.current) return;
    avisado.current = true;
    cuadroAviso.current = requestAnimationFrame(onListo);
  });
  // Si la escena se desmonta antes del cuadro, el aviso ya no tiene destino.
  useEffect(() => () => cancelAnimationFrame(cuadroAviso.current), []);

  // ── Colores de los tokens (se recalculan al cambiar de tema).
  const colores = useMemo(
    () => ({
      celda: colorToken("--filete"),
      seccion: colorToken("--filete-fuerte"),
      seleccion: colorToken("--seleccion"),
      ejes: [hexToken("--aviso"), hexToken("--acento"), hexToken("--tinta-2")] as [
        string,
        string,
        string,
      ],
      rotulo: hexToken("--superficie"),
      resalte: colorToken("--acento"),
      marco: colorToken("--foco"),
    }),
    // `tema` no se lee: solo marca cuándo hay que volver a leer los tokens.
    [tema],
  );
  useEffect(() => {
    corte.uColorMarca.value.copy(colores.seleccion);
    corte.uColorResalte.value.copy(colores.resalte);
    invalidate();
  }, [colores, corte, invalidate]);

  // Bloque pinchado en Modificar: tinte en el shader (un uniform, sin geometría).
  const pinchado = useEdicion((s) => s.pinchado);
  useEffect(() => {
    corte.uMarcar.value = pinchado ? 1 : 0;
    if (pinchado) corte.uCeldaMarcada.value.set(...pinchado.celda);
    invalidate();
  }, [pinchado, corte, invalidate]);

  // Cambios sueltos pendientes: capa con sus caras nuevas dentro de cada
  // malla (completa y cerrada), que se ve y se oculta con ella.
  useLayoutEffect(() => {
    const quitar = [montarCambios(completa, cambiosPrevia, corte, atlas, colores.marco)];
    if (cerrada) quitar.push(montarCambios(cerrada.g, cambiosPrevia, corte, atlas, colores.marco));
    invalidate();
    return () => quitar.forEach((q) => q());
  }, [completa, cerrada, cambiosPrevia, corte, atlas, colores, invalidate]);

  const verRendimiento = useVisor((s) => s.rendimiento);
  const [ancho, , largo] = malla.tamano;
  const radio = Math.hypot(...malla.tamano) / 2;

  return (
    <>
      <OrbitControls
        ref={controles}
        makeDefault
        enableDamping={!reducir}
        dampingFactor={0.12}
        autoRotate={girando}
        autoRotateSpeed={reducir ? VELOCIDAD_GIRO / 3 : VELOCIDAD_GIRO}
      />
      <primitive object={completa.grupo} dispose={null} />
      {cerrada && <primitive object={cerrada.g.grupo} dispose={null} />}
      <Grid
        position={[ancho / 2, -0.01, largo / 2]}
        args={[Math.ceil(ancho / 16) * 16 + 32, Math.ceil(largo / 16) * 16 + 32]}
        cellSize={1}
        cellThickness={0.6}
        cellColor={colores.celda}
        sectionSize={16}
        sectionThickness={1}
        sectionColor={colores.seccion}
        fadeDistance={Math.max(radio * 6, 64)}
        fadeStrength={1}
        side={DoubleSide}
      />
      <Seleccion color={colores.seleccion} />
      <GizmoHelper alignment="bottom-right" margin={MARGEN_GIZMO}>
        <GizmoViewport axisColors={colores.ejes} labelColor={colores.rotulo} />
      </GizmoHelper>
      {PanelRendimiento && verRendimiento && (
        <Suspense fallback={null}>
          <PanelRendimiento position="bottom-left" />
        </Suspense>
      )}
    </>
  );
}

/**
 * Caja de alambre sobre el bloque seleccionado, en el color de selección de
 * los tokens. Con prueba de profundidad, como en el juego: dibujada encima de
 * todo parecía flotar cuando el bloque estaba detrás de otro.
 */
function Seleccion({ color }: { color: Color }) {
  const seleccion = useVisor((s) => s.seleccion);
  const geometria = useMemo(() => {
    const caja = new BoxGeometry(1.01, 1.01, 1.01);
    const aristas = new EdgesGeometry(caja);
    caja.dispose();
    return aristas;
  }, []);
  const material = useMemo(() => new LineBasicMaterial({ toneMapped: false }), []);
  useEffect(
    () => () => {
      geometria.dispose();
      material.dispose();
    },
    [geometria, material],
  );
  material.color.copy(color);
  if (!seleccion) return null;
  const [x, y, z] = seleccion.celda;
  return (
    <lineSegments
      geometry={geometria}
      material={material}
      position={[x + 0.5, y + 0.5, z + 0.5]}
      renderOrder={10}
    />
  );
}

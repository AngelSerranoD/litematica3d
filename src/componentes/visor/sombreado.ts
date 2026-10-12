// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

// Materiales del visor: iluminación plana como la de Minecraft (un factor
// fijo por cara, sin luces ni sombras), color por material desde una textura
// de datos, corte por capas en el shader y el camino preparado para el atlas
// de texturas (formato y convenios en docs/visor.md, secciones 4 y 5).

import {
  DataTexture,
  FloatType,
  FrontSide,
  NearestFilter,
  NoColorSpace,
  RGBAFormat,
  ShaderMaterial,
  UnsignedByteType,
  Vector2,
  type IUniform,
  type Texture,
} from "three";
import type { MaterialMalla } from "../../api/visor";

/** Texels por fila de las tablas por material (color y celda del atlas). */
export const ANCHO_TABLA = 256;

/** Alfa del color plano de los materiales translúcidos (cristal, agua…). */
const ALFA_TRANSLUCIDO = 0.6;

/** Uniforms del corte por capas: los comparten todos los materiales del visor. */
export interface UniformsCorte {
  /** Primera capa visible (coordenadas del modelo). */
  uDesde: IUniform<number>;
  /** Última capa visible, inclusiva. */
  uHasta: IUniform<number>;
}

export const crearUniformsCorte = (): UniformsCorte => ({
  uDesde: { value: -1e9 },
  uHasta: { value: 1e9 },
});

/**
 * Atlas de texturas. `celda(textura)` devuelve el rectángulo de la textura útil
 * de esa celda en coordenadas UV del atlas: [u, v, ancho, alto], con v = 0
 * abajo (la imagen va volteada). El relleno de la celda queda fuera.
 */
export interface AtlasVisor {
  textura: Texture;
  celda: (textura: number) => [number, number, number, number];
  /** Tamaño del atlas en píxeles. */
  tamano: [number, number];
  /** Niveles de mipmap sin sangrado entre celdas (docs/visor.md 11.2). */
  nivelesMip: number;
}

const vertexShader = /* glsl */ `
attribute float normalIdx;
attribute vec2 uvBloque;
attribute float materialIdx;

uniform highp sampler2D uColores;
uniform highp sampler2D uCeldas;
uniform highp sampler2D uRotaciones;

varying vec2 vUv;
varying float vY;
flat varying vec4 vColor;
flat varying vec4 vCelda;
flat varying float vRot;
flat varying float vLuz;
flat varying float vNormalY;

// Luz fija por índice de normal (docs/visor.md 3.3): arriba 1, abajo 0,5,
// norte y sur 0,8, oeste y este 0,6, planos cruzados 0,8.
const float LUZ[10] = float[10](1.0, 0.5, 0.8, 0.8, 0.6, 0.6, 0.8, 0.8, 0.8, 0.8);
const float NORMAL_Y[10] = float[10](1.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);

void main() {
  int n = clamp(int(normalIdx + 0.5), 0, 9);
  int m = int(materialIdx + 0.5);
  ivec2 t = ivec2(m % ${ANCHO_TABLA}, m / ${ANCHO_TABLA});
  vLuz = LUZ[n];
  vNormalY = NORMAL_Y[n];
  vec4 c = texelFetch(uColores, t, 0);
  vColor = vec4(c.rgb * vLuz, c.a);
  vCelda = texelFetch(uCeldas, t, 0);
  vRot = texelFetch(uRotaciones, t, 0).r;
  // UV en bloques: la textura se repite una vez por bloque con fract().
  vUv = uvBloque / 16.0;
  vec4 mundo = modelMatrix * vec4(position, 1.0);
  vY = mundo.y;
  gl_Position = projectionMatrix * viewMatrix * mundo;
}
`;

const fragmentShader = /* glsl */ `
uniform float uDesde;
uniform float uHasta;
uniform sampler2D uAtlas;
uniform float uTieneAtlas;
uniform float uBordes;
uniform vec2 uTamAtlas;
uniform float uMaxLod;

varying vec2 vUv;
varying float vY;
flat varying vec4 vColor;
flat varying vec4 vCelda;
flat varying float vRot;
flat varying float vLuz;
flat varying float vNormalY;

// Giro de la textura (horario mirando desde fuera), docs/visor.md 11.2.
// rot: 0, 1, 2 o 3 cuartos de vuelta. La parte lineal (sin el +1 de los
// ejes invertidos) sirve para girar también las derivadas.
vec2 girar(vec2 l, float rot) {
  if (rot > 2.5) return vec2(l.y, 1.0 - l.x);
  if (rot > 1.5) return vec2(1.0 - l.x, 1.0 - l.y);
  if (rot > 0.5) return vec2(1.0 - l.y, l.x);
  return l;
}
vec2 girarDerivada(vec2 d, float rot) {
  if (rot > 2.5) return vec2(d.y, -d.x);
  if (rot > 1.5) return -d;
  if (rot > 0.5) return vec2(-d.y, d.x);
  return d;
}

// El PNG es sRGB y la textura lo declara: la GPU devuelve lineal. El color
// plano y la luz de Minecraft van en sRGB, así que se vuelve a codificar aquí.
vec3 aSRGB(vec3 c) {
  vec3 bajo = c * 12.92;
  vec3 alto = 1.055 * pow(max(c, vec3(0.0)), vec3(1.0 / 2.4)) - 0.055;
  return mix(bajo, alto, step(vec3(0.0031308), c));
}

void main() {
  // Corte por capas: la capa del fragmento es la del bloque al que pertenece
  // la cara (se retrocede contra la normal, como en el picking).
  float capa = floor(vY - vNormalY * 0.001);
  if (capa < uDesde || capa > uHasta) discard;

  vec4 color = vColor;
  if (uTieneAtlas > 0.5 && vCelda.z > 0.0) {
    // Con textura: celda del material + posición dentro del bloque. Las
    // derivadas se toman de la UV continua para que fract() no rompa el mipmap.
    vec2 local = girar(fract(vUv), vRot);
    vec2 gx = girarDerivada(dFdx(vUv), vRot);
    vec2 gy = girarDerivada(dFdy(vUv), vRot);
    // LOD limitado a los niveles sin sangrado entre celdas: si el pixel
    // cubre más texels que 2^uMaxLod se acortan las derivadas.
    vec2 gxPx = gx * vCelda.zw * uTamAtlas;
    vec2 gyPx = gy * vCelda.zw * uTamAtlas;
    float rho = max(length(gxPx), length(gyPx));
    float tope = exp2(uMaxLod);
    float k = rho > tope ? tope / rho : 1.0;
    vec4 tex = textureGrad(
      uAtlas,
      vCelda.xy + local * vCelda.zw,
      gx * vCelda.zw * k,
      gy * vCelda.zw * k
    );
    // Translúcidos: el alfa de la propia textura si lo trae; los texels
    // opacos (agua, hielo…) usan el alfa del material.
    float alfa = tex.a;
#ifdef TRANSLUCIDO
    if (alfa > 0.99) alfa = vColor.a;
#endif
    color = vec4(aSRGB(tex.rgb) * vLuz, alfa);
  } else if (uBordes > 0.0) {
    // Color plano: un filete fino en el borde de cada bloque para que se
    // distingan; se apaga a lo lejos, donde solo haría ruido.
    vec2 local = fract(vUv);
    vec2 paso = max(fwidth(vUv), vec2(1e-5));
    vec2 d = min(local, 1.0 - local) / paso;
    float linea = 1.0 - clamp(min(d.x, d.y) - 0.5, 0.0, 1.0);
    float cerca = 1.0 - smoothstep(0.12, 0.35, max(paso.x, paso.y));
    color.rgb *= 1.0 - 0.16 * linea * cerca * uBordes;
  }

#ifdef TRANSLUCIDO
  if (color.a < 0.01) discard;
  gl_FragColor = color;
#else
  // Opacos y de recorte en la misma pasada: alphaTest a 0,5.
  if (color.a < 0.5) discard;
  gl_FragColor = vec4(color.rgb, 1.0);
#endif
}
`;

/** Materiales de una malla: los dos de la pasada y sus tablas. */
export interface MaterialesVisor {
  /** Opacos y de recorte (grupo 0 de cada chunk). */
  opaco: ShaderMaterial;
  /** Translúcidos (grupo 1): mezcla alfa, sin escribir profundidad, al final. */
  translucido: ShaderMaterial;
  /** RGBA8 por material: color de respaldo y alfa de la pasada. */
  colores: DataTexture;
  /** RGBA32F por material: celda del atlas [u, v, ancho, alto]; ancho 0 = sin textura. */
  celdas: DataTexture;
  /** RGBA32F por material: cuartos de vuelta de la textura (0..3) en el canal rojo. */
  rotaciones: DataTexture;
  dispose: () => void;
}

/** Textura de 1 × 1 para que `uAtlas` siempre tenga algo enlazado. */
let atlasVacio: DataTexture | null = null;
function texturaVacia(): DataTexture {
  if (!atlasVacio) {
    atlasVacio = new DataTexture(new Uint8Array([255, 255, 255, 255]), 1, 1);
    atlasVacio.needsUpdate = true;
  }
  return atlasVacio;
}

function tablaDatos(datos: Uint8Array | Float32Array, filas: number): DataTexture {
  const t = new DataTexture(
    datos,
    ANCHO_TABLA,
    filas,
    RGBAFormat,
    datos instanceof Float32Array ? FloatType : UnsignedByteType,
  );
  t.magFilter = NearestFilter;
  t.minFilter = NearestFilter;
  t.generateMipmaps = false;
  t.colorSpace = NoColorSpace;
  t.needsUpdate = true;
  return t;
}

/**
 * Rellena la tabla de celdas con el atlas (o la vacía si `atlas` es null) y
 * conecta la textura. Se puede llamar otra vez cuando llegue o cambie el atlas.
 */
export function aplicarAtlas(
  m: MaterialesVisor,
  materiales: MaterialMalla[],
  atlas: AtlasVisor | null,
): void {
  const datos = m.celdas.image.data as Float32Array;
  const giros = m.rotaciones.image.data as Float32Array;
  datos.fill(0);
  giros.fill(0);
  if (atlas) {
    materiales.forEach((mat, i) => {
      if (mat.textura === null) return;
      datos.set(atlas.celda(mat.textura), i * 4);
      giros[i * 4] = (Math.round(mat.rotacion / 90) % 4 + 4) % 4;
    });
  }
  m.celdas.needsUpdate = true;
  m.rotaciones.needsUpdate = true;
  for (const material of [m.opaco, m.translucido]) {
    material.uniforms.uAtlas.value = atlas?.textura ?? texturaVacia();
    material.uniforms.uTieneAtlas.value = atlas ? 1 : 0;
    if (atlas) {
      material.uniforms.uTamAtlas.value.set(atlas.tamano[0], atlas.tamano[1]);
      material.uniforms.uMaxLod.value = atlas.nivelesMip;
    }
    // Con texturas sobran los filetes de bloque.
    material.uniforms.uBordes.value = atlas ? 0 : 1;
  }
}

/**
 * Crea los materiales de una malla. `corte` se comparte (no se copia): al
 * cambiar sus valores cambian todos los materiales a la vez.
 */
export function crearMateriales(
  materiales: MaterialMalla[],
  corte: UniformsCorte,
  atlas: AtlasVisor | null = null,
): MaterialesVisor {
  const filas = Math.max(1, Math.ceil(materiales.length / ANCHO_TABLA));
  const rgba = new Uint8Array(ANCHO_TABLA * filas * 4);
  materiales.forEach((mat, i) => {
    rgba.set(mat.color, i * 4);
    rgba[i * 4 + 3] = mat.tipo === "translucido" ? Math.round(ALFA_TRANSLUCIDO * 255) : 255;
  });
  const colores = tablaDatos(rgba, filas);
  const celdas = tablaDatos(new Float32Array(ANCHO_TABLA * filas * 4), filas);
  const rotaciones = tablaDatos(new Float32Array(ANCHO_TABLA * filas * 4), filas);

  const crear = (translucido: boolean) =>
    new ShaderMaterial({
      vertexShader,
      fragmentShader,
      defines: translucido ? { TRANSLUCIDO: "" } : {},
      uniforms: {
        uDesde: corte.uDesde,
        uHasta: corte.uHasta,
        uColores: { value: colores },
        uCeldas: { value: celdas },
        uRotaciones: { value: rotaciones },
        uTamAtlas: { value: new Vector2(1, 1) },
        uMaxLod: { value: 0 },
        uAtlas: { value: texturaVacia() },
        uTieneAtlas: { value: 0 },
        uBordes: { value: 1 },
      },
      side: FrontSide,
      transparent: translucido,
      depthWrite: !translucido,
    });

  const m: MaterialesVisor = {
    opaco: crear(false),
    translucido: crear(true),
    colores,
    celdas,
    rotaciones,
    dispose: () => {
      m.opaco.dispose();
      m.translucido.dispose();
      colores.dispose();
      celdas.dispose();
      rotaciones.dispose();
    },
  };
  aplicarAtlas(m, materiales, atlas);
  return m;
}

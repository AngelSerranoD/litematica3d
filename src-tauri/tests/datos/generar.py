# Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
# Ver LICENSE y NOTICE.
#
# Genera los schematics de prueba que usan los tests de Rust del parser, y un
# `esperados.json` con los valores correctos, calculados desde la propia lista de
# bloques que se escribe en cada archivo (nunca a mano).
#
# Dependencias (solo para GENERAR datos; no forman parte de la app):
#     pip install nbtlib litemapy
#   - nbtlib   (MIT)
#   - litemapy (GPL-3): se usa únicamente como herramienta externa e independiente
#     para escribir los .litematic; no se enlaza ni se distribuye con la app.
#
# Ejecución (desde cualquier carpeta; escribe junto al script):
#     python generar.py
#
# Los datos son deterministas (semillas fijas, fechas fijas): regenerar no cambia
# el contenido. Al final se ejecuta `verificar()`, que relee cada archivo con una
# decodificación independiente (y con litemapy para los .litematic) y comprueba
# dimensiones, total_bloques y muestras contra esperados.json.

import gzip
import json
import math
import random
import sys
from pathlib import Path

import nbtlib
import numpy as np
from nbtlib import ByteArray, Compound, File, Int, List, Long, Short, String

from litemapy import BlockState, Region, Schematic

RUTA = Path(__file__).resolve().parent
LEGADO = RUTA.parent.parent / "datos" / "legado_bloques.json"

DATA_VERSION = 3953          # Minecraft 1.21
FECHA_MS = 1767225600000     # 2026-01-01T00:00:00Z, fija para que sea reproducible
AIRES = {"minecraft:air", "minecraft:cave_air", "minecraft:void_air"}


# ---------------------------------------------------------------- utilidades de estado

def estado(nombre, props=None):
    """Texto canónico de un block state: propiedades ordenadas por clave."""
    if not props:
        return nombre
    return nombre + "[" + ",".join(f"{k}={props[k]}" for k in sorted(props)) + "]"


def parsear(texto):
    """'minecraft:x[a=1,b=2]' -> ('minecraft:x', {'a': '1', 'b': '2'})."""
    if "[" not in texto:
        return texto, {}
    nombre, resto = texto.split("[", 1)
    props = dict(p.split("=", 1) for p in resto.rstrip("]").split(",") if p)
    return nombre, props


def canonico(texto):
    return estado(*parsear(texto))


def es_aire(texto):
    return texto.split("[", 1)[0] in AIRES


def indice(w, l, x, y, z):
    """Orden común a todos los formatos: x + z*ancho + y*ancho*largo."""
    return x + z * w + y * w * l


# Conjunto global de 20 estados (aire y cave_air incluidos) del que salen las paletas.
ESTADOS = [canonico(s) for s in [
    "minecraft:air",
    "minecraft:stone",
    "minecraft:cave_air",
    "minecraft:oak_stairs[facing=north,half=bottom,shape=straight,waterlogged=false]",
    "minecraft:oak_planks",
    "minecraft:cobblestone",
    "minecraft:red_wool",
    "minecraft:glass",
    "minecraft:oak_log[axis=y]",
    "minecraft:dirt",
    "minecraft:grass_block[snowy=false]",
    "minecraft:spruce_slab[type=top,waterlogged=false]",
    "minecraft:sandstone",
    "minecraft:oak_door[facing=east,half=lower,hinge=left,open=false,powered=false]",
    "minecraft:lantern[hanging=false,waterlogged=false]",
    "minecraft:gold_block",
    "minecraft:diamond_block",
    "minecraft:birch_planks",
    "minecraft:stone_bricks",
    "minecraft:deepslate_bricks",
]]


def rellenar(w, h, l, pool, semilla, peso_aire=0.25, peso_cave=0.03):
    """Lista plana de estados (orden x + z*w + y*w*l). Mezcla aire intercalado y
    garantiza que TODAS las entradas de `pool` aparecen al menos una vez."""
    rng = random.Random(semilla)
    aire, cave = pool[0], pool[2] if len(pool) > 2 and pool[2] == "minecraft:cave_air" else None
    resto = [s for s in pool if not es_aire(s)]
    celdas = []
    for _ in range(w * h * l):
        r = rng.random()
        if r < peso_aire:
            celdas.append(aire)
        elif cave and r < peso_aire + peso_cave:
            celdas.append(cave)
        else:
            celdas.append(rng.choice(resto))
    posiciones = list(range(len(celdas)))
    rng.shuffle(posiciones)
    for pos, s in zip(posiciones, pool):
        celdas[pos] = s
    return celdas


def contar_bloques(celdas):
    return sum(1 for s in celdas if s is not None and not es_aire(s))


def elegir_muestras(region, w, h, l, celdas, maximo=6):
    """4-6 posiciones (relativas a la esquina mínima de la región) con su estado.
    Una posición sin declarar (None) cuenta como minecraft:air."""
    def en(x, y, z):
        s = celdas[indice(w, l, x, y, z)]
        return "minecraft:air" if s is None else s

    candidatas = [(0, 0, 0), (w - 1, h - 1, l - 1)]
    vistos = set(candidatas)
    criterios = [
        lambda s: "[" in s and not s.startswith("?"),   # con propiedades
        lambda s: s == "minecraft:cave_air",
        lambda s: s == "minecraft:air",
        lambda s: s == "minecraft:stone",
    ]
    for crit in criterios:
        for y in range(h):
            for z in range(l):
                for x in range(w):
                    if (x, y, z) not in vistos and crit(en(x, y, z)):
                        candidatas.append((x, y, z))
                        vistos.add((x, y, z))
                        break
                else:
                    continue
                break
            else:
                continue
            break
    muestras = []
    for (x, y, z) in candidatas:
        s = en(x, y, z)
        if s.startswith("?"):          # id sin nombre conocido: no se comprueba el nombre
            continue
        muestras.append({"region": region, "x": x, "y": y, "z": z, "bloque": s})
    return muestras[:maximo]


# ---------------------------------------------------------------- guardado NBT

def guardar_nbt(ruta, raiz, nombre_raiz="", gzip_activo=True):
    File(raiz, gzipped=gzip_activo, root_name=nombre_raiz).save(ruta)


def a_bytes_nbt(valores):
    """Lista de enteros 0..255 -> ByteArray (con signo, como en NBT)."""
    return ByteArray(np.array(valores, dtype=np.uint8).astype(np.int8))


def varint(n):
    out = bytearray()
    while True:
        b = n & 0x7F
        n >>= 7
        if n:
            out.append(b | 0x80)
        else:
            out.append(b)
            return bytes(out)


def leer_varints(datos):
    res, valor, desp = [], 0, 0
    for b in datos:
        valor |= (b & 0x7F) << desp
        if b & 0x80:
            desp += 7
        else:
            res.append(valor)
            valor, desp = 0, 0
    return res


# ---------------------------------------------------------------- 1 y 2: litematic

def region_litemapy(pos, tam, celdas):
    """Crea una Region de litemapy desde la cuadrícula en coordenadas relativas a la
    esquina mínima. litemapy usa, para tamaños negativos, coordenadas de región
    desplazadas: x_region = x_min_relativa + (tam + 1) cuando tam < 0."""
    w, h, l = (abs(t) for t in tam)
    reg = Region(*pos, *tam)

    def conv(v, t):
        return v + t + 1 if t < 0 else v

    for y in range(h):
        for z in range(l):
            for x in range(w):
                s = celdas[indice(w, l, x, y, z)]
                if s == "minecraft:air":
                    continue
                nombre, props = parsear(s)
                reg[conv(x, tam[0]), conv(y, tam[1]), conv(z, tam[2])] = BlockState(nombre, **props)
    return reg


def esquina_minima(pos, tam):
    return tuple(p + (t + 1 if t < 0 else 0) for p, t in zip(pos, tam))


def esperado_litematic(nombre, autor, regiones):
    """regiones: lista de (pos, tam, celdas) en el orden en que se guardan."""
    minimos, maximos = [], []
    for pos, tam, _ in regiones:
        mn = esquina_minima(pos, tam)
        minimos.append(mn)
        maximos.append(tuple(m + abs(t) for m, t in zip(mn, tam)))
    caja = [max(m[i] for m in maximos) - min(m[i] for m in minimos) for i in range(3)]
    muestras = []
    for i, (pos, tam, celdas) in enumerate(regiones):
        w, h, l = (abs(t) for t in tam)
        muestras += elegir_muestras(i, w, h, l, celdas, maximo=6 // len(regiones))
    return {
        "formato": "litematic",
        "ancho": caja[0], "alto": caja[1], "largo": caja[2],
        "total_bloques": sum(contar_bloques(c) for _, _, c in regiones),
        "regiones": len(regiones),
        "nombre": nombre, "autor": autor,
        "version_datos": DATA_VERSION,
        "muestras": muestras,
    }


def generar_litematic_simple(esperados):
    tam = (7, 6, 8)                                  # 336 bloques, paleta de 20 -> 5 bits
    celdas = rellenar(*tam, ESTADOS, semilla=101)
    reg = region_litemapy((0, 0, 0), tam, celdas)
    s = Schematic(name="Casita de prueba", author="Ángel Serrano",
                  description="Una región, paleta de 20 estados (5 bits)", mc_version=DATA_VERSION)
    s.regions["principal"] = reg
    s.save(str(RUTA / "litematic_simple.litematic"))
    esperados["litematic_simple.litematic"] = esperado_litematic(
        "Casita de prueba", "Ángel Serrano", [((0, 0, 0), tam, celdas)])


def generar_litematic_multi(esperados):
    # Región A: tamaño NEGATIVO en x (-5), 11 estados (4 bits). Región B: positiva, 8 estados (3 bits).
    pos_a, tam_a = (0, 0, 0), (-5, 3, 4)
    pool_a = ESTADOS[0:11]
    pos_b, tam_b = (20, 2, -10), (3, 4, 5)
    pool_b = ESTADOS[0:2] + ESTADOS[11:17]
    ca = rellenar(*(abs(t) for t in tam_a), pool_a, semilla=202)
    cb = rellenar(*(abs(t) for t in tam_b), pool_b, semilla=203)
    s = Schematic(name="Dos regiones", author="Ángel Serrano",
                  description="Region con tamaño negativo + region separada", mc_version=DATA_VERSION)
    s.regions["negativa"] = region_litemapy(pos_a, tam_a, ca)
    s.regions["positiva"] = region_litemapy(pos_b, tam_b, cb)
    s.save(str(RUTA / "litematic_multi.litematic"))
    esperados["litematic_multi.litematic"] = esperado_litematic(
        "Dos regiones", "Ángel Serrano", [(pos_a, tam_a, ca), (pos_b, tam_b, cb)])


# ---------------------------------------------------------------- 3, 4 y 9: Sponge

def sponge_datos(w, h, l, pool, semilla):
    """Devuelve (paleta {estado: idx}, celdas, bytes de BlockData). La paleta sigue el
    orden de `pool` y se escribe completa."""
    celdas = rellenar(w, h, l, pool, semilla)
    paleta = {s: i for i, s in enumerate(pool)}
    datos = b"".join(varint(paleta[s]) for s in celdas)
    return paleta, celdas, datos


def esperado_simple(formato, nombre, autor, version, w, h, l, celdas):
    return {
        "formato": formato, "ancho": w, "alto": h, "largo": l,
        "total_bloques": contar_bloques(celdas), "regiones": 1,
        "nombre": nombre, "autor": autor, "version_datos": version,
        "muestras": elegir_muestras(0, w, h, l, celdas),
    }


def pool_sponge_v2():
    """135 entradas: 5 especiales + 130 `test_n` (algunas con propiedades) -> índices > 127."""
    pool = [ESTADOS[0], ESTADOS[1], ESTADOS[2], ESTADOS[3], ESTADOS[18]]
    for n in range(130):
        pool.append(estado(f"minecraft:test_{n}", {"level": str(n % 4)} if n % 3 == 0 else None))
    return pool


def raiz_sponge_v2(w, h, l, paleta, datos, nombre, autor):
    raiz = Compound()
    raiz["Version"] = Int(2)
    raiz["DataVersion"] = Int(DATA_VERSION)
    raiz["Width"], raiz["Height"], raiz["Length"] = Short(w), Short(h), Short(l)
    raiz["PaletteMax"] = Int(len(paleta))
    raiz["Palette"] = Compound({String(k): Int(v) for k, v in paleta.items()})
    raiz["BlockData"] = a_bytes_nbt(list(datos))
    raiz["Metadata"] = Compound({
        "Name": String(nombre), "Author": String(autor), "Date": Long(FECHA_MS)})
    return raiz


def generar_sponge_v2(esperados):
    w, h, l = 10, 6, 7                               # 420 bloques, paleta de 135
    paleta, celdas, datos = sponge_datos(w, h, l, pool_sponge_v2(), semilla=301)
    assert max(leer_varints(datos)) > 127 and len(datos) > w * h * l, "faltan varints de 2 bytes"
    guardar_nbt(RUTA / "sponge_v2.schem", raiz_sponge_v2(w, h, l, paleta, datos,
                                                         "Sponge v2 grande", "Ángel Serrano"), "Schematic")
    esperados["sponge_v2.schem"] = esperado_simple(
        "sponge", "Sponge v2 grande", "Ángel Serrano", DATA_VERSION, w, h, l, celdas)


def generar_sponge_v3(esperados):
    w, h, l = 4, 3, 5
    pool = [ESTADOS[0], ESTADOS[1], ESTADOS[3], ESTADOS[6], ESTADOS[14]]   # 5 entradas -> 3 bits
    paleta, celdas, datos = sponge_datos(w, h, l, pool, semilla=401)
    esquema = Compound()
    esquema["Version"] = Int(3)
    esquema["DataVersion"] = Int(DATA_VERSION)
    esquema["Width"], esquema["Height"], esquema["Length"] = Short(w), Short(h), Short(l)
    esquema["Metadata"] = Compound({
        "Name": String("Sponge v3 pequeño"), "Author": String("Ángel Serrano"), "Date": Long(FECHA_MS)})
    esquema["Blocks"] = Compound({
        "Palette": Compound({String(k): Int(v) for k, v in paleta.items()}),
        "Data": a_bytes_nbt(list(datos)),
        "BlockEntities": List[Compound](),
    })
    # raíz con nombre vacío que contiene el compound «Schematic»
    guardar_nbt(RUTA / "sponge_v3.schem", Compound({"Schematic": esquema}), "")
    esperados["sponge_v3.schem"] = esperado_simple(
        "sponge", "Sponge v3 pequeño", "Ángel Serrano", DATA_VERSION, w, h, l, celdas)


def generar_sponge_como_schematic(esperados):
    w, h, l = 3, 3, 3
    pool = [ESTADOS[0], ESTADOS[1], ESTADOS[4], ESTADOS[7]]
    paleta, celdas, datos = sponge_datos(w, h, l, pool, semilla=901)
    guardar_nbt(RUTA / "sponge_como_schematic.schematic",
                raiz_sponge_v2(w, h, l, paleta, datos, "Sponge con extension legada", "Ángel Serrano"),
                "Schematic")
    esperados["sponge_como_schematic.schematic"] = esperado_simple(
        "sponge", "Sponge con extension legada", "Ángel Serrano", DATA_VERSION, w, h, l, celdas)


# ---------------------------------------------------------------- 5: MCEdit / WorldEdit antiguo

def cargar_legado():
    return json.loads(LEGADO.read_text(encoding="utf-8"))


def nombre_legado(tabla, bid, meta):
    """Estado esperado de un id:meta; '?id:meta' si no está en la tabla (id desconocido)."""
    s = tabla.get(f"{bid}:{meta}")
    return canonico(s) if s is not None else f"?{bid}:{meta}"


def generar_mcedit(esperados):
    tabla = cargar_legado()
    w, h, l = 6, 4, 5                                # 120 bloques
    pool_ids = [(0, 0), (1, 0), (35, 14), (5, 2), (4, 0), (17, 0), (53, 1), (300, 0), (4095, 5)]
    for bid, meta in pool_ids:
        if bid <= 255:
            assert f"{bid}:{meta}" in tabla, f"{bid}:{meta} no está en legado_bloques.json"
    estados = [nombre_legado(tabla, b, m) for b, m in pool_ids]
    por_estado = dict(zip(estados, pool_ids))
    # el relleno trabaja con estados (texto); el aire es la entrada 0
    celdas = rellenar(w, h, l, estados, semilla=501, peso_cave=0)
    # id > 255 forzados en un índice par (4) y uno impar (7) para probar ambos nibbles de AddBlocks
    celdas[4] = estados[7]
    celdas[7] = estados[8]
    # se vuelve a garantizar que no falte ninguna entrada tras sobrescribir
    faltan = [s for s in estados if s not in celdas]
    assert not faltan, f"entradas ausentes: {faltan}"

    n = w * h * l
    ids = [por_estado[s][0] for s in celdas]
    metas = [por_estado[s][1] for s in celdas]
    add = bytearray((n + 1) // 2)
    for i, bid in enumerate(ids):
        alto = (bid >> 8) & 0x0F
        add[i >> 1] |= alto if i % 2 == 0 else (alto << 4)
    assert any(b > 255 for b in ids) and any(ids[i] > 255 for i in range(0, n, 2)) \
        and any(ids[i] > 255 for i in range(1, n, 2))

    raiz = Compound()
    raiz["Width"], raiz["Height"], raiz["Length"] = Short(w), Short(h), Short(l)
    raiz["Materials"] = String("Alpha")
    raiz["Blocks"] = a_bytes_nbt([b & 0xFF for b in ids])
    raiz["Data"] = a_bytes_nbt(metas)
    raiz["AddBlocks"] = a_bytes_nbt(list(add))
    raiz["Entities"] = List[Compound]()
    raiz["TileEntities"] = List[Compound]()
    guardar_nbt(RUTA / "mcedit.schematic", raiz, "Schematic")

    # los ids sin nombre (estado '?...') cuentan como bloque NO aire
    esperados["mcedit.schematic"] = esperado_simple(
        "mcedit", None, None, None, w, h, l, celdas)


# ---------------------------------------------------------------- 6 y 7: estructura (Structure Block)

def generar_estructura(esperados):
    w, h, l = 5, 4, 6                                # 120 posiciones
    pool = [ESTADOS[0], ESTADOS[1], ESTADOS[3], ESTADOS[4], ESTADOS[6], ESTADOS[7]]
    celdas = rellenar(w, h, l, pool, semilla=601, peso_cave=0)
    # posiciones sin declarar (None): una de cada 6, sin tocar la cobertura de la paleta
    declaradas_necesarias = {}
    for i, s in enumerate(celdas):
        declaradas_necesarias.setdefault(s, []).append(i)
    for s, idxs in declaradas_necesarias.items():
        # el primero de cada estado se conserva siempre
        for i in idxs[1:]:
            if i % 6 == 5:
                celdas[i] = None
    assert any(c is None for c in celdas)
    assert all(s in celdas for s in pool)

    def entrada_paleta(s):
        nombre, props = parsear(s)
        e = Compound({"Name": String(nombre)})
        if props:
            e["Properties"] = Compound({String(k): String(v) for k, v in props.items()})
        return e

    idx = {s: i for i, s in enumerate(pool)}
    bloques = List[Compound]()
    for y in range(h):
        for z in range(l):
            for x in range(w):
                s = celdas[indice(w, l, x, y, z)]
                if s is None:
                    continue
                bloques.append(Compound({
                    "pos": List[Int]([Int(x), Int(y), Int(z)]),
                    "state": Int(idx[s]),
                }))

    def raiz(paletas):
        r = Compound()
        r["DataVersion"] = Int(DATA_VERSION)
        r["size"] = List[Int]([Int(w), Int(h), Int(l)])
        if paletas is None:
            r["palette"] = List[Compound]([entrada_paleta(s) for s in pool])
        else:
            r["palettes"] = List[List[Compound]](
                [List[Compound]([entrada_paleta(s) for s in p]) for p in paletas])
        r["blocks"] = bloques
        r["entities"] = List[Compound]()
        return r

    # variante de aleatoriedad: la segunda paleta cambia piedra por adoquín y planks por cristal
    segunda = [("minecraft:cobblestone" if s == "minecraft:stone" else s) for s in pool]
    guardar_nbt(RUTA / "estructura.nbt", raiz(None))
    guardar_nbt(RUTA / "estructura_palettes.nbt", raiz([pool, segunda]))
    guardar_nbt(RUTA / "sin_comprimir.nbt", raiz(None), gzip_activo=False)

    esp = esperado_simple("estructura", None, None, DATA_VERSION, w, h, l, celdas)
    for nombre in ("estructura.nbt", "estructura_palettes.nbt", "sin_comprimir.nbt"):
        esperados[nombre] = json.loads(json.dumps(esp))


# ---------------------------------------------------------------- 8: archivos inválidos

def generar_invalidos(esperados):
    datos = (RUTA / "litematic_simple.litematic").read_bytes()
    (RUTA / "corrupto.litematic").write_bytes(datos[: len(datos) // 2])
    basura = random.Random(12345).randbytes(200)
    assert basura[:2] != b"\x1f\x8b"
    (RUTA / "basura.schem").write_bytes(basura)
    esperados["corrupto.litematic"] = {"error": True}
    esperados["basura.schem"] = {"error": True}


# ---------------------------------------------------------------- verificación independiente

def _texto_estado(entrada):
    props = {str(k): str(v) for k, v in entrada.get("Properties", {}).items()}
    return estado(str(entrada["Name"]), props)


def _desempaquetar_litematic(longs, n, nbits):
    """Desempaqueta valores de `nbits` bits sobre una secuencia de longs (sin huecos)."""
    palabras = [int(v) & 0xFFFFFFFFFFFFFFFF for v in longs]
    mascara = (1 << nbits) - 1
    res = []
    for i in range(n):
        bit = i * nbits
        p, d = bit >> 6, bit & 63
        v = palabras[p] >> d
        if d + nbits > 64:
            v |= palabras[p + 1] << (64 - d)
        res.append(v & mascara)
    return res


def _leer(ruta):
    return nbtlib.load(str(ruta))


def _decodificar(nombre, esp):
    """Devuelve (ancho, alto, largo, total, regiones, getter(region,x,y,z), nombre, autor, version)."""
    f = _leer(RUTA / nombre)
    fmt = esp["formato"]
    if fmt == "litematic":
        meta = f["Metadata"]
        regs = []
        for rn, r in f["Regions"].items():
            tam = [int(r["Size"][k]) for k in "xyz"]
            pos = [int(r["Position"][k]) for k in "xyz"]
            w, h, l = (abs(t) for t in tam)
            paleta = [_texto_estado(e) for e in r["BlockStatePalette"]]
            nbits = max(math.ceil(math.log2(len(paleta))), 2)
            vals = _desempaquetar_litematic(r["BlockStates"], w * h * l, nbits)
            regs.append((pos, tam, [paleta[v] for v in vals], len(paleta)))
        mins = [esquina_minima(p, t) for p, t, _, _ in regs]
        maxs = [tuple(m + abs(t) for m, t in zip(mn, tm)) for mn, (_, tm, _, _) in zip(mins, regs)]
        caja = [max(m[i] for m in maxs) - min(m[i] for m in mins) for i in range(3)]
        total = sum(1 for _, _, c, _ in regs for s in c if not es_aire(s))

        def get(rg, x, y, z):
            _, tam, c, _ = regs[rg]
            return c[indice(abs(tam[0]), abs(tam[2]), x, y, z)]
        return (*caja, total, len(regs), get, str(meta["Name"]), str(meta["Author"]),
                int(f["MinecraftDataVersion"]))

    if fmt == "sponge":
        raiz = f["Schematic"] if "Schematic" in f else f
        v3 = int(raiz["Version"]) == 3
        w, h, l = int(raiz["Width"]), int(raiz["Height"]), int(raiz["Length"])
        paleta_nbt = raiz["Blocks"]["Palette"] if v3 else raiz["Palette"]
        datos = raiz["Blocks"]["Data"] if v3 else raiz["BlockData"]
        inversa = {int(v): str(k) for k, v in paleta_nbt.items()}
        idxs = leer_varints(bytes(int(b) & 0xFF for b in datos))
        assert len(idxs) == w * h * l
        celdas = [inversa[i] for i in idxs]
        total = sum(1 for s in celdas if not es_aire(s))
        return (w, h, l, total, 1, lambda rg, x, y, z: celdas[indice(w, l, x, y, z)],
                str(raiz["Metadata"]["Name"]), str(raiz["Metadata"]["Author"]), int(raiz["DataVersion"]))

    if fmt == "mcedit":
        tabla = cargar_legado()
        w, h, l = int(f["Width"]), int(f["Height"]), int(f["Length"])
        blocks = [int(b) & 0xFF for b in f["Blocks"]]
        datos = [int(b) & 0x0F for b in f["Data"]]
        add = [int(b) & 0xFF for b in f["AddBlocks"]]
        celdas = []
        for i, b in enumerate(blocks):
            alto = (add[i >> 1] & 0x0F) if i % 2 == 0 else ((add[i >> 1] & 0xF0) >> 4)
            celdas.append(nombre_legado(tabla, b | (alto << 8), datos[i]))
        total = sum(1 for s in celdas if not es_aire(s))
        return (w, h, l, total, 1, lambda rg, x, y, z: celdas[indice(w, l, x, y, z)], None, None, None)

    # estructura
    w, h, l = (int(v) for v in f["size"])
    paleta_nbt = f["palette"] if "palette" in f else f["palettes"][0]
    paleta = [_texto_estado(e) for e in paleta_nbt]
    celdas = ["minecraft:air"] * (w * h * l)
    for b in f["blocks"]:
        x, y, z = (int(v) for v in b["pos"])
        celdas[indice(w, l, x, y, z)] = paleta[int(b["state"])]
    total = sum(1 for s in celdas if not es_aire(s))
    return (w, h, l, total, 1, lambda rg, x, y, z: celdas[indice(w, l, x, y, z)],
            None, None, int(f["DataVersion"]))


def verificar():
    esperados = json.loads((RUTA / "esperados.json").read_text(encoding="utf-8"))
    errores = []
    for nombre, esp in esperados.items():
        if esp.get("error"):
            try:
                _leer(RUTA / nombre)
            except Exception:
                continue
            errores.append(f"{nombre}: debía fallar al leerse y se leyó")
            continue
        w, h, l, total, regiones, get, nom, aut, ver = _decodificar(nombre, esp)
        comprobar = {"ancho": w, "alto": h, "largo": l, "total_bloques": total,
                     "regiones": regiones, "nombre": nom, "autor": aut, "version_datos": ver}
        for clave, valor in comprobar.items():
            if esp[clave] != valor:
                errores.append(f"{nombre}: {clave} esperado {esp[clave]!r}, leído {valor!r}")
        for m in esp["muestras"]:
            leido = get(m["region"], m["x"], m["y"], m["z"])
            if canonico(leido) != m["bloque"]:
                errores.append(f"{nombre}: muestra {m} -> leído {leido}")
        if not (4 <= len(esp["muestras"]) <= 6):
            errores.append(f"{nombre}: {len(esp['muestras'])} muestras (se piden 4-6)")

        # litemapy como segundo lector independiente de los .litematic
        if esp["formato"] == "litematic":
            s = Schematic.load(str(RUTA / nombre))
            if len(s.regions) != esp["regiones"]:
                errores.append(f"{nombre}: litemapy lee {len(s.regions)} regiones")
            tot = 0
            for rg, (rn, reg) in enumerate(s.regions.items()):
                tam = (reg.width, reg.height, reg.length)
                for m in (m for m in esp["muestras"] if m["region"] == rg):
                    c = tuple(m[k] + (t + 1 if t < 0 else 0) for k, t in zip("xyz", tam))
                    b = reg[c]
                    if canonico(estado(b.id, dict(b.properties()))) != m["bloque"]:
                        errores.append(f"{nombre}: litemapy {m} -> {b}")
                for x in range(abs(tam[0])):
                    for y in range(abs(tam[1])):
                        for z in range(abs(tam[2])):
                            c = (x + (tam[0] + 1 if tam[0] < 0 else 0),
                                 y + (tam[1] + 1 if tam[1] < 0 else 0),
                                 z + (tam[2] + 1 if tam[2] < 0 else 0))
                            if not es_aire(reg[c].id):
                                tot += 1
            if tot != esp["total_bloques"]:
                errores.append(f"{nombre}: litemapy cuenta {tot} bloques, esperado {esp['total_bloques']}")
            print(f"  litemapy enclosure de {nombre}: {s.width}x{s.height}x{s.length} "
                  f"(esperado {esp['ancho']}x{esp['alto']}x{esp['largo']})")
        print(f"  ok {nombre}: {esp['ancho']}x{esp['alto']}x{esp['largo']}, "
              f"{esp['total_bloques']} bloques, {esp['regiones']} region(es)")
    if errores:
        print("VERIFICACION FALLIDA:")
        for e in errores:
            print("  -", e)
        return False
    print("VERIFICACION CORRECTA")
    return True


def main():
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    esperados = {}
    generar_litematic_simple(esperados)
    generar_litematic_multi(esperados)
    generar_sponge_v2(esperados)
    generar_sponge_v3(esperados)
    generar_mcedit(esperados)
    generar_estructura(esperados)
    generar_invalidos(esperados)
    generar_sponge_como_schematic(esperados)
    (RUTA / "esperados.json").write_text(
        json.dumps(esperados, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"Generados {len(esperados)} archivos en {RUTA}")
    return 0 if verificar() else 1


if __name__ == "__main__":
    sys.exit(main())

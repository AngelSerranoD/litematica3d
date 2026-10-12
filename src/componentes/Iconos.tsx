// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

/*
 * Iconos de trazo de 1 px dibujados sobre una rejilla de 12 px, como los
 * botones de la barra de título. Toman el color del texto (currentColor) y
 * son decorativos: el nombre accesible lo pone el botón que los contiene.
 */

const comunes = {
  width: 12,
  height: 12,
  viewBox: "0 0 12 12",
  "aria-hidden": true,
  className: "icono",
} as const;

export function IconoPapelera() {
  return (
    <svg {...comunes}>
      <path d="M1 2.5H11M4.5 2.5V0.5H7.5V2.5M2.5 2.5V11.5H9.5V2.5M5 5V9M7 5V9" />
    </svg>
  );
}

export function IconoBuscar() {
  return (
    <svg {...comunes}>
      <rect x="0.5" y="0.5" width="7" height="7" />
      <path d="M7.5 7.5L11.5 11.5" />
    </svg>
  );
}

export function IconoRejilla() {
  return (
    <svg {...comunes}>
      <path d="M0.5 0.5H11.5V11.5H0.5ZM6 0.5V11.5M0.5 6H11.5" />
    </svg>
  );
}

export function IconoLista() {
  return (
    <svg {...comunes}>
      <path d="M0.5 0.5H11.5V11.5H0.5ZM0.5 4.17H11.5M0.5 7.83H11.5M3.5 0.5V11.5" />
    </svg>
  );
}

/** Flecha del sentido de orden: hacia arriba = ascendente. */
export function IconoSentido({ ascendente }: { ascendente: boolean }) {
  return (
    <svg {...comunes} width={8} height={12} viewBox="0 0 8 12">
      {ascendente ? (
        <path d="M4 11.5V1M0.5 4.5L4 1L7.5 4.5" />
      ) : (
        <path d="M4 0.5V11M0.5 7.5L4 11L7.5 7.5" />
      )}
    </svg>
  );
}

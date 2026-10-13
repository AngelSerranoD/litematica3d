// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import "./SaltarAlContenido.css";

/**
 * Enlace «Saltar al contenido»: invisible hasta que recibe el foco (primer
 * Tab). Lleva el foco al contenido principal de la pantalla actual, marcado
 * con `id="contenido-principal"`: en la biblioteca, el elemento activo de la
 * rejilla; en el schematic, el lienzo del visor (o el contenedor si aún no
 * hay lienzo).
 */
export function SaltarAlContenido() {
  const saltar = (e: React.MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault();
    const zona = document.getElementById("contenido-principal");
    if (!zona) return;
    const destino =
      zona.querySelector<HTMLElement>('[data-abrir][tabindex="0"]') ??
      zona.querySelector<HTMLElement>("canvas") ??
      zona;
    destino.focus();
  };
  return (
    <a className="saltar" href="#contenido-principal" onClick={saltar}>
      Saltar al contenido
    </a>
  );
}

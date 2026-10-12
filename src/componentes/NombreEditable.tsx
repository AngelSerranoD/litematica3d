// Litematica3D — © 2026 Ángel Serrano Domínguez. Todos los derechos reservados.
// Ver LICENSE y NOTICE.

import { useEffect, useRef } from "react";

/** Igual que en el núcleo (MAX_NOMBRE en src-tauri/src/biblioteca.rs). */
const MAX_NOMBRE = 200;

interface Props {
  nombre: string;
  className?: string;
  /**
   * `nombre` null = cancelar. `devolverFoco` es verdadero cuando se termina
   * con el teclado (Intro o Esc) y falso cuando se sale con un clic fuera:
   * en ese caso el foco ya está donde el usuario lo ha puesto.
   */
  alTerminar: (nombre: string | null, devolverFoco: boolean) => void;
}

/** Campo de renombrado en el sitio: Intro guarda, Esc cancela, salir guarda. */
export function NombreEditable({ nombre, className, alTerminar }: Props) {
  const campo = useRef<HTMLInputElement>(null);
  // Intro y luego el blur del desmontaje no deben guardar dos veces.
  const terminado = useRef(false);

  useEffect(() => {
    campo.current?.focus({ preventScroll: true });
    campo.current?.select();
  }, []);

  const terminar = (valor: string | null, devolverFoco: boolean) => {
    if (terminado.current) return;
    terminado.current = true;
    const limpio = valor?.trim() ?? "";
    alTerminar(limpio && limpio !== nombre ? limpio : null, devolverFoco);
  };

  return (
    <input
      ref={campo}
      className={className}
      type="text"
      defaultValue={nombre}
      maxLength={MAX_NOMBRE}
      spellCheck={false}
      autoComplete="off"
      aria-label="Nuevo nombre del schematic"
      onKeyDown={(e) => {
        // Las teclas del campo no deben llegar a la navegación de la rejilla.
        e.stopPropagation();
        if (e.key === "Enter") {
          e.preventDefault();
          terminar(e.currentTarget.value, true);
        } else if (e.key === "Escape") {
          // preventDefault: el Esc global de App no debe hacer nada más.
          e.preventDefault();
          terminar(null, true);
        }
      }}
      onBlur={(e) => terminar(e.currentTarget.value, false)}
      onClick={(e) => e.stopPropagation()}
      onDoubleClick={(e) => e.stopPropagation()}
    />
  );
}

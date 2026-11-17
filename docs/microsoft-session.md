# Sesión Microsoft persistente

## Comportamiento

La aplicación Rust conserva una sola sesión Microsoft por directorio de datos.
Tras completar el flujo PKCE:

1. El perfil y el token de acceso de Minecraft permanecen únicamente en memoria.
2. El token de renovación se guarda en el almacén de credenciales del sistema,
   acompañado del Client ID público y un número de esquema.
3. Al iniciar en modo Microsoft se recupera esa credencial y se solicita un acceso
   nuevo. Si Microsoft rota el token de renovación, se reemplaza el guardado.
4. El acceso se renueva antes de lanzar Minecraft o modificar skins cuando alcanza
   su plazo de renovación: el menor entre 45 minutos y la expiración JWT menos
   un minuto. La lectura de `exp` solo organiza la renovación, no autentica al usuario.

En modo local no se consulta el llavero ni Microsoft al arrancar. Al seleccionar
Microsoft se intenta restaurar la sesión. No se cambia el perfil local activo.
Microsoft puede revocar o rechazar la renovación; en ese caso hay que autenticar
otra vez. No se promete una sesión indefinida ni autenticación sin conexión.

## Almacenamiento

- Linux: backend **Secret Service** de keyring, con GNOME Keyring o KWallet.
- Windows: backend nativo **WinVaultKeyring**.
- macOS: backend nativo **Keyring** de Keychain.
- No se usa la selección automática de keyring ni backends de archivos en texto plano.

El servicio se identifica como `SkyLauncher/Microsoft/<hash-del-directorio-de-datos>`.
La entrada `session` contiene solo esquema, Client ID y refresh token.
Los directorios de QA definidos con `MI_LAUNCHER_DATA` usan otra entrada para no
leer o reemplazar la sesión real.

`~/.mi-minecraft-launcher/session-state.json` contiene exclusivamente un esquema y
dos indicadores públicos: `restore` y `cleanup_pending`. No contiene credenciales.
Sus escrituras son atómicas; se rechazan enlaces simbólicos.
Los tokens nunca se envían al frontend ni se incluyen en mensajes de error,
perfiles, ajustes, paquetes o snapshots de fuentes.

Copiar los datos del launcher a otro equipo no exporta la sesión Microsoft.
Desinstalar el paquete no borra automáticamente las entradas del llavero: utiliza
Cerrar sesión y olvidar antes si quieres retirarlas.

## Fallos y controles de la interfaz

| Situación | Resultado |
|---|---|
| Llavero disponible | Se guarda y restaura la sesión |
| Llavero bloqueado, no instalado o sin proveedor | Aviso explícito; no hay fallback inseguro |
| Guardado fallido tras login | Cuenta válida solo en esa ejecución; Guardar sesión permite reintentar |
| Error temporal de red al restaurar | Se conserva la credencial; Restaurar sesión guardada permite reintentar |
| Token revocado o cuenta sin acceso a Minecraft | Se desactiva la restauración y se solicita autenticar de nuevo |
| Otro Client ID | No se renueva la credencial de otra aplicación |
| Cerrar sesión con llavero bloqueado | Se limpia la memoria y se desactiva primero la restauración en disco; se avisa del borrado pendiente |

Olvidar sesión guardada permite reintentar el borrado tras desbloquear el llavero.
Si no se puede escribir el indicador que desactiva la restauración, no se comunica
un cierre de sesión exitoso.
Iniciar un acceso nuevo desactiva antes la restauración de la credencial anterior,
evitando que un fallo al guardar termine restaurando otra cuenta al reiniciar.
El cierre de sesión afecta al launcher, no a las cookies del navegador ni a otras aplicaciones.

Los ayudantes de autenticación de minecraft-launcher-lib 8.0 no aceptan timeout.
El backend, que procesa las acciones secuencialmente, utiliza un adaptador HTTP
limitado a su módulo de autenticación: conexión de 10 s y lectura de 20 s por petición.
Siempre se restaura el módulo y no se modifica requests globalmente.

## Preparar Linux

En Arch:

```sh
sudo pacman -S --needed python-keyring python-secretstorage
```

Debe existir un proveedor de Secret Service en la sesión de escritorio:
GNOME Keyring o KWallet con ese servicio habilitado. Usa un llavero protegido
con contraseña y desbloquéalo cuando el escritorio lo solicite.
En escritorios mínimos como Hyprland puede ser necesario configurar el inicio
del proveedor o su integración PAM; el launcher no modifica esa configuración.

Al ejecutar desde fuentes:

```sh
.venv/bin/python -m pip install -r requirements.txt
cargo run --release --locked
```

El paquete Arch `0.3.0-2` incorpora las nuevas dependencias y actualiza el
ejecutable; la versión anterior no puede guardar la sesión. Después de actualizar,
cierra y vuelve a abrir la aplicación e inicia sesión una vez.

## Archivos y verificación

- `backend.py`: almacén seguro, restauración, renovación y cierre de sesión.
- `src/main.rs`: avisos y controles de recuperación; solo recibe nombre y estado público.
- `requirements.txt` y `packaging/arch/PKGBUILD`: dependencias.
- `tests/test_session.py`: llavero simulado, sin tocar secretos reales.

```sh
.venv/bin/python -m unittest discover -s tests -v
cargo test --locked
cargo clippy --locked -- -D warnings
```

Las pruebas cubren login con PKCE, reinicio, rotación, expiración, revocación,
llavero bloqueado, caída de red, no filtrar tokens, namespaces por directorio,
selección explícita del backend seguro y cierre de sesión con borrado pendiente.
Los layouts se prueban también a 800×600.
La autenticación y renovación reales requieren probar con la cuenta del usuario;
no se han solicitado, inspeccionado ni utilizado sus credenciales para estas pruebas.

La interfaz heredada `launcher.py` no cambia y sigue conservando la sesión solo
durante su ejecución.

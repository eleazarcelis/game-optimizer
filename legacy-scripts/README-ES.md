# 🎮 Optimizador de Juegos y Lanzadores

Un potente script de Windows que aplica optimizaciones específicas a juegos, lanzadores y procesos auxiliares para maximizar el rendimiento y minimizar el consumo de recursos.

## 📋 Descripción General

Este optimizador proporciona tres perfiles distintos adaptados para diferentes tipos de aplicaciones:
- **Perfil Lanzador**: Optimización balanceada para plataformas de juegos (Steam, Epic, Origin, etc.)
- **Perfil Juego**: Optimización de máximo rendimiento para juegos
- **Perfil Proceso Secundario**: Configuración estable para anticheats y procesos auxiliares

## ✨ Características

### 🚀 Perfil Lanzador
- **Configuración GPU**: Ajustes balanceados para eficiencia energética
- **Prioridad CPU**: Baja para nuevas ejecuciones del lanzador (script en español)
- **GameBar**: Habilitado para overlays y captura de pantalla
- **Plan de Energía**: Modo balanceado para consumo óptimo de energía
- **Impacto**: Fluidez mejorada sin uso extra de recursos

### 🎯 Perfil Juego
- **Configuración GPU**: Modo alto rendimiento (siempre usa GPU dedicada)
- **Prioridad CPU**: Alta para nuevas ejecuciones del juego (script en español)
- **Windows Defender**: Excluye carpeta del juego y ejecutable del escaneo en tiempo real
- **GameDVR**: Deshabilitado para eliminar lag de grabación
- **GameBar**: Habilitado para activación automática del Modo Juego
- **Plan de Energía**: Alto rendimiento (CPU y GPU a máxima capacidad)
- **Impacto**: Máximos FPS, latencia reducida, experiencia de juego sin interrupciones

### 🛡️ Perfil Proceso Secundario
- **Configuración GPU**: Ajustes balanceados (no interfiere con el juego principal)
- **Prioridad CPU**: No se modifica
- **GameBar**: Deshabilitado para evitar conflictos con anticheats
- **Plan de Energía**: Modo balanceado (no compite por recursos del sistema)
- **Impacto**: Máxima estabilidad, previene falsos positivos con sistemas anticheat

## 🔄 Opciones de Reversión

Las opciones de reversión aplican valores predefinidos a las configuraciones existentes. En el script en español, la prioridad de CPU restaura el valor anterior guardado; si no existe una copia, conserva el valor actual:
- **Revertir Lanzador**: Elimina todas las optimizaciones específicas del lanzador
- **Revertir Juego**: Elimina todas las optimizaciones de juego y restaura configuraciones de seguridad predeterminadas
- **Revertir Proceso Secundario**: Restaura configuración predeterminada para procesos auxiliares

## 🛠️ Uso

1. **Ejecuta el script apropiado**:
   - `game-optimizer-en.bat` (versión en inglés)
   - `game-optimizer-es.bat` (versión en español)

2. **Selecciona tu perfil deseado** (1-6):
   - Opciones 1-3: Configurar optimizaciones
   - Opciones 4-6: Revertir configuraciones

3. **Proporciona la ruta completa** del ejecutable que deseas optimizar

4. **Permisos de administrador** son necesarios para configurar o revertir la prioridad de CPU del lanzador o juego, y para las optimizaciones del Perfil Juego

## ⚠️ Notas Importantes

- **Todos los cambios son PERMANENTES** hasta que se reviertan manualmente usando las opciones de reversión
- **El Perfil Juego requiere privilegios de Administrador** para exclusiones de Windows Defender y optimizaciones a nivel de sistema
- En el script en español, la prioridad de CPU usa Image File Execution Options, un mecanismo de Windows no documentado para este uso. Se aplica a futuras ejecuciones por nombre de archivo, también en otras carpetas; no cambia procesos ya iniciados.
- Mantén `cpu-priority.ps1` junto a `game-optimizer-es.bat`. La copia de la prioridad anterior se guarda en el registro del equipo y se conserva al volver a configurar el mismo nombre.
- **Siempre respalda tu sistema** antes de aplicar optimizaciones
- **Prueba las configuraciones** con tus juegos específicos y configuración de hardware

## 🖥️ Requisitos del Sistema

- **Sistema Operativo**: Windows 10 (probado)
- **PowerShell**: Requerido para configuraciones avanzadas
- **Derechos de Administrador**: Requeridos para optimizaciones del Perfil Juego

## ⚡ Beneficios de Rendimiento

### Perfil Lanzador
- Tiempo de inicio del lanzador reducido
- Menor uso de recursos en segundo plano
- Capacidad de respuesta del sistema mantenida

### Perfil Juego
- **Aumento de FPS**: Mejora del 5-15% en la mayoría de juegos
- **Lag de Entrada Reducido**: Menor latencia para juegos competitivos
- **Eliminación de Interrupciones**: Sin escaneos de Windows Defender durante el juego
- **Entrega de Energía Optimizada**: CPU y GPU funcionan a máximo rendimiento

### Perfil Proceso Secundario
- **Compatibilidad con Anticheat**: Previene conflictos con sistemas de protección de juegos
- **Estabilidad del Sistema**: Mantiene asignación balanceada de recursos
- **Sin Falsos Positivos**: Reduce problemas de detección de anticheat

## 🔧 Detalles Técnicos

El optimizador modifica los siguientes componentes de Windows:
- **Preferencias GPU**: Entradas de registro DirectX UserGpuPreferences
- **Game Config Store**: Configuraciones del Modo Juego de Windows
- **Windows Defender**: Exclusiones de protección en tiempo real
- **Gestión de Energía**: Configuraciones del plan de energía del sistema
- **Prioridad CPU**: Image File Execution Options y copia del valor anterior en el registro (script en español)
- **GameDVR**: Características de grabación de juegos de Windows

## 📝 Información de Versión

- **Versión**: 1.0.0
- **Autor**: Ele
- **GitHub**: https://github.com/eleazarcelis
- **Probado En**: Windows 10

## ⚠️ Descargo de Responsabilidad

Esta herramienta ha sido **probada exclusivamente en Windows 10**. La compatibilidad con otras versiones de Windows no está garantizada. Úsala bajo tu propio riesgo y siempre crea puntos de restauración del sistema antes de aplicar optimizaciones.

## 🤝 Contribuir

Siéntete libre de reportar problemas, sugerir mejoras o contribuir al proyecto a través del repositorio de GitHub.

## 📄 Licencia

Este proyecto es de código abierto. Por favor, consulta el repositorio para detalles de la licencia.
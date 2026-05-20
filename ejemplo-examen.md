# Examen: ejemplo

## Pregunta 1 (Sí o No)

¿Es un contenedor lo mismo que una máquina virtual?

**Respuesta:** No. Una máquina virtual incluye su propio sistema operativo completo, mientras que un contenedor no lo requiere, lo que lo hace más ligero.

## Pregunta 2 (Sí o No)

¿Puede una imagen de Docker ser modificada una vez que ya fue construida?

**Respuesta:** No. Una imagen es un plano de solo lectura, lo que significa que su contenido no puede cambiarse una vez creado.

## Pregunta 3 (Sí o No)

¿Es el Dockerfile un archivo necesario para construir una imagen de Docker?

**Respuesta:** Sí. Es el archivo de texto donde se escriben todas las instrucciones necesarias para crear la imagen.

## Pregunta 4 (Sí o No)

¿Es Docker Compose la herramienta adecuada para ejecutar aplicaciones que necesitan más de un contenedor al mismo tiempo?

**Respuesta:** Sí. Esta herramienta está diseñada específicamente para definir y poner en marcha aplicaciones que funcionan con múltiples contenedores coordinados.

## Pregunta 5 (Escenario)

¿Cómo se utiliza el comando 'docker run' para poner en marcha una aplicación?

**Respuesta:** Primero abres la terminal, luego escribes 'docker run' seguido del nombre de la imagen que deseas ejecutar y presionas la tecla Enter.

## Pregunta 6 (Sí o No)

¿Cómo se realiza el proceso para crear una imagen a partir de un Dockerfile?

**Respuesta:** Debes ubicarte en la carpeta donde está el archivo Dockerfile desde tu terminal y ejecutar el comando 'docker build' para que la plataforma procese las instrucciones y genere la imagen.

## Pregunta 7 (Escenario)

Imagina que eres un desarrollador y quieres que tu aplicación ocupe el menor espacio posible en el disco duro. ¿Qué harías con respecto a la elección de la imagen base?

**Respuesta:** Elegiría una imagen base pequeña, como Alpine, para asegurarme de que la aplicación final sea más ligera y eficiente.


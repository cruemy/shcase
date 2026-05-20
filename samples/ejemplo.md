---
main_title: "Introducción a Docker"
secondary_title: "Contenedores para desarrolladores"
---

## ¿Qué es Docker?

Docker es una plataforma que permite ejecutar aplicaciones en contenedores.
Un contenedor es como una caja que incluye todo lo necesario para que una
aplicación funcione: código, librerías, variables de entorno y archivos de
configuración.

A diferencia de una máquina virtual, un contenedor no incluye un sistema
operativo completo. Comparte el kernel del host, lo que lo hace mucho más
liviano y rápido de iniciar.

## Contenedores vs Máquinas Virtuales

Una máquina virtual incluye su propio sistema operativo invitado. Esto significa
que cada VM consume varios gigabytes de disco y minutos para arrancar.

Un contenedor comparte el kernel del sistema operativo anfitrión. Esto hace que
ocupe megabytes y arranque en segundos.

La desventaja es que los contenedores están atados al kernel del host: no podés
correr un contenedor Linux en Windows sin una capa de virtualización adicional.

## Imágenes y Contenedores

Una imagen es un plano o plantilla de solo lectura. Definís qué va adentro:
una base Ubuntu, Node.js 20, tu código, etc.

Un contenedor es una instancia en ejecución de esa imagen. Podés tener múltiples
contenedores corriendo desde la misma imagen, cada uno con su propio estado.

Las imágenes se construyen con un Dockerfile y se almacenan en registries como
Docker Hub.

## El Dockerfile

El Dockerfile es un archivo de texto con instrucciones para construir una imagen.

```
FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
CMD ["node", "index.js"]
```

Cada instrucción crea una capa. Docker cachea las capas, así que si solo cambia
el código fuente, las capas de instalación de dependencias se reusan.

## Comandos esenciales

- `docker build -t nombre .` — construye una imagen desde un Dockerfile
- `docker run nombre` — crea y ejecuta un contenedor desde una imagen
- `docker ps` — lista contenedores en ejecución
- `docker images` — lista imágenes descargadas
- `docker pull imagen` — descarga una imagen de un registry
- `docker push imagen` — sube una imagen a un registry

## Docker Compose

Docker Compose permite definir y ejecutar aplicaciones multi-contenedor.
Con un archivo `compose.yaml` definís servicios, redes y volúmenes.

Ejemplo: una app web con Node.js que usa PostgreSQL y Redis. Con un solo
comando `docker compose up` se levantan los tres servicios.

Compose es ideal para entornos de desarrollo porque configura toda la
infraestructura con un archivo YAML.

## Buenas prácticas

1. Usá imágenes base pequeñas como Alpine para reducir el tamaño final.
2. Ordená las instrucciones del Dockerfile de menos a más cambiantes para
   maximizar el caché de capas.
3. No corras el contenedor como root. Usá `USER` en el Dockerfile.
4. Usá `.dockerignore` para excluir archivos innecesarios del contexto de build.
5. Etiquetá las imágenes con versiones semánticas, no solo `latest`.

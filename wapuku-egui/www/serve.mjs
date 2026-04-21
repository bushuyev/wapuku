import { readFile, stat } from "node:fs/promises";
import { createServer } from "node:https";
import { extname, join, normalize, resolve } from "node:path";

const scriptDir = resolve(import.meta.dirname);
const distDir = resolve(scriptDir, "dist");
const certPath = "/home/bu/dvl/localhost-dev-certs/localhost.crt";
const keyPath = "/home/bu/dvl/localhost-dev-certs/localhost.key";
const host = process.env.HOST ?? "127.0.0.1";
const port = Number.parseInt(process.env.PORT ?? "7777", 10);

const contentTypes = new Map([
  [".css", "text/css; charset=utf-8"],
  [".html", "text/html; charset=utf-8"],
  [".ico", "image/x-icon"],
  [".js", "text/javascript; charset=utf-8"],
  [".json", "application/json; charset=utf-8"],
  [".svg", "image/svg+xml"],
  [".txt", "text/plain; charset=utf-8"],
  [".wasm", "application/wasm"],
]);

function toFilePath(urlPath) {
  const decodedPath = decodeURIComponent(urlPath.split("?")[0]);
  const relativePath =
    decodedPath === "/" || decodedPath === ""
      ? "index.html"
      : normalize(decodedPath).replace(/^(\.\.(\/|\\|$))+/, "").replace(/^[/\\]+/, "");

  return resolve(join(distDir, relativePath));
}

function isSpaRoute(urlPath) {
  const decodedPath = decodeURIComponent(urlPath.split("?")[0]);
  return decodedPath === "/" || !decodedPath.slice(decodedPath.lastIndexOf("/") + 1).includes(".");
}

async function sendFile(response, filePath, method) {
  const body = await readFile(filePath);
  response.writeHead(200, {
    "Cache-Control": "no-store",
    "Content-Length": body.length,
    "Content-Type": contentTypes.get(extname(filePath)) ?? "application/octet-stream",
    "Cross-Origin-Embedder-Policy": "require-corp",
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Resource-Policy": "same-origin",
    "Origin-Agent-Cluster": "?1",
  });

  if (method !== "HEAD") {
    response.end(body);
    return;
  }

  response.end();
}

async function handler(request, response) {
  try {
    const method = request.method ?? "GET";
    if (method !== "GET" && method !== "HEAD") {
      response.writeHead(405, { Allow: "GET, HEAD" });
      response.end("Method Not Allowed");
      return;
    }

    let filePath = toFilePath(request.url ?? "/");
    if (!filePath.startsWith(distDir)) {
      response.writeHead(403);
      response.end("Forbidden");
      return;
    }

    try {
      const fileStat = await stat(filePath);
      if (fileStat.isDirectory()) {
        filePath = resolve(filePath, "index.html");
      }
      await sendFile(response, filePath, method);
      return;
    } catch (error) {
      if (!isSpaRoute(request.url ?? "/")) {
        throw error;
      }
    }

    await sendFile(response, resolve(distDir, "index.html"), method);
  } catch {
    response.writeHead(404, {
      "Cache-Control": "no-store",
      "Content-Type": "text/plain; charset=utf-8",
      "Cross-Origin-Embedder-Policy": "require-corp",
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Resource-Policy": "same-origin",
      "Origin-Agent-Cluster": "?1",
    });
    response.end("Not found");
  }
}

const [cert, key] = await Promise.all([readFile(certPath), readFile(keyPath)]);

const server = createServer({ cert, key }, handler);

server.on("error", (error) => {
  console.error(`Failed to start HTTPS server on ${host}:${port}`);
  console.error(error);
  process.exit(1);
});

server.listen(port, host, () => {
  console.log(`Serving ${distDir} at https://localhost:${port}/`);
});

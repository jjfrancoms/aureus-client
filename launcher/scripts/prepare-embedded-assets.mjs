import { copyFile, mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const launcherDirectory = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repositoryDirectory = resolve(launcherDirectory, "..");
const outputDirectory = resolve(repositoryDirectory, "outputs");

const assets = [
  {
    name: "fabric-api-0.141.6+1.21.11.jar",
    url: "https://maven.fabricmc.net/net/fabricmc/fabric-api/fabric-api/0.141.6+1.21.11/fabric-api-0.141.6+1.21.11.jar",
  },
  {
    name: "fabric-installer-1.1.2.jar",
    url: "https://maven.fabricmc.net/net/fabricmc/fabric-installer/1.1.2/fabric-installer-1.1.2.jar",
  },
];

await mkdir(outputDirectory, { recursive: true });
await copyFile(
  resolve(repositoryDirectory, "build/libs/aureus-client-0.3.0.jar"),
  resolve(outputDirectory, "aureus-client-0.3.0-minecraft-1.21.11.jar"),
);

for (const asset of assets) {
  const response = await fetch(asset.url);
  if (!response.ok) throw new Error(`No se pudo descargar ${asset.name}: HTTP ${response.status}`);
  await writeFile(resolve(outputDirectory, asset.name), Buffer.from(await response.arrayBuffer()));
}

console.log("Recursos integrados de Aureus preparados.");

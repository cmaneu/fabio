import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const defaultSchema = resolve(scriptDirectory, "../../src/commands/context/data/agent/commands.json");
const defaultOutput = resolve(scriptDirectory, "../src/content/docs/reference/commands");

export function inlineCode(value) {
  return `\`${String(value).replaceAll("`", "\\`")}\``;
}

export function renderGroup(groupName, group) {
  const lines = [
    "---",
    `title: ${JSON.stringify(groupName)}`,
    `description: ${JSON.stringify(group.description ?? `Commands for ${groupName}.`)}`,
    "---",
    "",
    `# ${inlineCode(`fabio ${groupName}`)}`,
    "",
    group.description ?? `Commands for ${groupName}.`,
    "",
    `**Authentication scope:** ${inlineCode(group.auth_scope ?? "fabric")}`,
    "",
  ];

  for (const [commandName, command] of Object.entries(group.subcommands ?? {})) {
    lines.push(`## ${inlineCode(commandName)}`, "", command.description ?? "No description available.", "");

    const flags = Object.entries(command.flags ?? {});
    const requiredFlags = flags.filter(([, flag]) => flag.required).map(([name]) => `${name} <value>`);
    const usage = [`fabio ${groupName} ${commandName}`, ...requiredFlags].join(" ");
    lines.push("```bash", usage, "```", "");

    if (flags.length > 0) {
      lines.push("| Flag | Type | Required | Description |", "| --- | --- | :---: | --- |");
      for (const [flagName, flag] of flags) {
        lines.push(
          `| ${inlineCode(flagName)} | ${inlineCode(flag.type ?? "string")} | ${flag.required ? "Yes" : "No"} | ${String(flag.description ?? "").replaceAll("|", "\\|")} |`,
        );
      }
      lines.push("");
    }

    if (Array.isArray(command.examples) && command.examples.length > 0) {
      lines.push("**Examples**", "");
      for (const example of command.examples) {
        lines.push("```bash", example, "```", "");
      }
    }

    const traits = [
      command.mutates ? "Mutates state" : "Read-only",
      command.destructive ? "Destructive" : null,
      command.async ? "Long-running operation" : null,
      command.returns ? `Returns ${command.returns}` : null,
    ].filter(Boolean);
    lines.push(`_${traits.join(" · ")}_`, "");
  }

  return `${lines.join("\n")}\n`;
}

export async function generateReference(schemaPath = defaultSchema, outputDirectory = defaultOutput) {
  const schema = JSON.parse(await readFile(schemaPath, "utf8"));
  await rm(outputDirectory, { recursive: true, force: true });
  await mkdir(outputDirectory, { recursive: true });

  const groups = Object.entries(schema).sort(([left], [right]) => left.localeCompare(right));
  for (const [groupName, group] of groups) {
    await writeFile(resolve(outputDirectory, `${groupName}.md`), renderGroup(groupName, group), "utf8");
  }

  return groups.length;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const count = await generateReference();
  console.log(`Generated reference for ${count} command groups.`);
}

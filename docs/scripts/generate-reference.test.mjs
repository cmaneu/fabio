import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { escapeHtml, generateReference, inlineCode, renderGroup } from "./generate-reference.mjs";

test("inlineCode escapes backticks", () => {
  assert.equal(inlineCode("a`b"), "`a\\`b`");
});

test("escapeHtml preserves placeholders as visible text", () => {
  assert.equal(escapeHtml("https://<id>/<name>?a=1&b=2"), "https://&lt;id&gt;/&lt;name&gt;?a=1&amp;b=2");
});

test("renderGroup includes command metadata and flags", () => {
  const markdown = renderGroup("workspace", {
    description: "Manage workspaces",
    auth_scope: "fabric",
    subcommands: {
      create: {
        description: "Create a workspace",
        flags: {
          "--name": { type: "string", required: true, description: "Workspace <name>" },
        },
        mutates: true,
        returns: "object",
        examples: ['fabio workspace create --name "Analytics"'],
      },
    },
  });

  assert.match(markdown, /fabio workspace create --name <value>/);
  assert.match(markdown, /\| `--name` \| `string` \| Yes \| Workspace &lt;name&gt; \|/);
  assert.match(markdown, /Mutates state · Returns object/);
});

test("generateReference creates one sorted page per group", async () => {
  const directory = await mkdtemp(join(tmpdir(), "fabio-reference-"));
  const schemaPath = join(directory, "commands.json");
  const outputPath = join(directory, "output");
  await writeFile(
    schemaPath,
    JSON.stringify({
      workspace: { description: "Workspaces", subcommands: {} },
      auth: { description: "Authentication", subcommands: {} },
    }),
  );

  const count = await generateReference(schemaPath, outputPath);

  assert.equal(count, 2);
  assert.match(await readFile(join(outputPath, "auth.md"), "utf8"), /title: "auth"/);
  assert.match(await readFile(join(outputPath, "workspace.md"), "utf8"), /title: "workspace"/);
});

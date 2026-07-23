import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

const [owner, repository] = (process.env.GITHUB_REPOSITORY ?? "").split("/");
const site = process.env.SITE_URL ?? (owner ? `https://${owner}.github.io` : "http://localhost:4321");
const base = process.env.BASE_PATH ?? (repository ? `/${repository}` : "/");

export default defineConfig({
  site,
  base,
  integrations: [
    starlight({
      title: "Fabio",
      description: "Agent-native command line interface for Microsoft Fabric.",
      favicon: "/favicon.svg",
      customCss: ["./src/styles/docs.css"],
      social: {
        github: "https://github.com/iemejia/fabio",
      },
      editLink: {
        baseUrl: "https://github.com/iemejia/fabio/edit/main/docs/",
      },
      sidebar: [
        {
          label: "Tutorials",
          items: [{ label: "Getting started", slug: "getting-started" }],
        },
        {
          label: "How-to guides",
          autogenerate: { directory: "guides" },
        },
        {
          label: "Explanation",
          autogenerate: { directory: "explanation" },
        },
        {
          label: "Reference",
          items: [
            { label: "CLI overview", slug: "reference" },
            { label: "Global flags", slug: "reference/global-flags" },
            {
              label: "Commands",
              autogenerate: { directory: "reference/commands" },
              collapsed: true,
            },
          ],
        },
      ],
      pagefind: true,
      head: [
        {
          tag: "meta",
          attrs: { name: "theme-color", content: "#0f6cbd" },
        },
      ],
    }),
  ],
});

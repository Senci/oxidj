import { readFileSync } from 'node:fs';
// Count whitespace-delimited visible text, including shared navigation and footer.
for (const page of ['index', 'roadmap']) {
  const text = readFileSync(
    new URL(`../${process.argv[2] || 'dist'}/${page}.html`, import.meta.url),
    'utf8',
  )
    .replace(
      /<head[\s\S]*?<\/head>|<script[\s\S]*?<\/script>|<style[\s\S]*?<\/style>|<!--[\s\S]*?-->/g,
      '',
    )
    .replace(/<[^>]+>/g, ' ');
  console.log(`${page}: ${text.trim().split(/\s+/).length} words`);
}

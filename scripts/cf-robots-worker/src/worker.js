// Cloudflare Worker — serves a permissive robots.txt at /robots.txt on every
// zone it's routed to. Everything else passes through to the origin.
//
// Deploy:
//   cd scripts/cf-robots-worker
//   wrangler deploy
//
// Attach routes (after first deploy):
//   bash ../attach_robots_routes.sh

const ROBOTS = `# Maximum-permissive crawler policy across all skworld zones.
# We explicitly welcome every AI / search / answer-engine / social crawler.
# Surface us everywhere.

User-agent: *
Allow: /

User-agent: Googlebot
Allow: /

User-agent: Bingbot
Allow: /

User-agent: DuckDuckBot
Allow: /

User-agent: GPTBot
Allow: /

User-agent: ChatGPT-User
Allow: /

User-agent: OAI-SearchBot
Allow: /

User-agent: ClaudeBot
Allow: /

User-agent: Claude-Web
Allow: /

User-agent: anthropic-ai
Allow: /

User-agent: PerplexityBot
Allow: /

User-agent: Perplexity-User
Allow: /

User-agent: Google-Extended
Allow: /

User-agent: GoogleOther
Allow: /

User-agent: Applebot
Allow: /

User-agent: Applebot-Extended
Allow: /

User-agent: Amazonbot
Allow: /

User-agent: CCBot
Allow: /

User-agent: Bytespider
Allow: /

User-agent: cohere-ai
Allow: /

User-agent: cohere-training-data-crawler
Allow: /

User-agent: Diffbot
Allow: /

User-agent: FacebookBot
Allow: /

User-agent: meta-externalagent
Allow: /

User-agent: ImagesiftBot
Allow: /

User-agent: Omgilibot
Allow: /

User-agent: PetalBot
Allow: /

User-agent: Timpibot
Allow: /

User-agent: facebookexternalhit
Allow: /

User-agent: Twitterbot
Allow: /

User-agent: LinkedInBot
Allow: /

User-agent: Discordbot
Allow: /

User-agent: TelegramBot
Allow: /

User-agent: WhatsApp
Allow: /

User-agent: Slackbot
Allow: /

User-agent: redditbot
Allow: /
`;

export default {
  async fetch(request) {
    const url = new URL(request.url);
    if (url.pathname === "/robots.txt") {
      return new Response(ROBOTS, {
        status: 200,
        headers: {
          "content-type": "text/plain; charset=utf-8",
          "cache-control": "public, max-age=3600",
          "x-served-by": "cf-permissive-robots-worker",
        },
      });
    }
    return fetch(request);
  },
};

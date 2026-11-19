/**
 * The end-to-end tests' ports, in one place for the Playwright config and the test helpers. They
 * differ from `npm run dev:web` (1430), so both can run at once without touching each other's data.
 */
export const SERVER_PORT = Number(process.env.FRIENDS_E2E_SERVER_PORT || 1440);
export const WEB_PORT = Number(process.env.FRIENDS_E2E_WEB_PORT || 1441);

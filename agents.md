# Browser Tool Policy

Strictly forbidden: use any browser automation, browser control, or browser inspection tools other than the configured Chrome DevTools integration for this environment.

Allowed:
- Configured `chrome-devtools` / Chrome DevTools MCP only.

Forbidden:
- Playwright
- Selenium
- Puppeteer
- WebDriver
- Browser-use style agents
- Direct CDP scripts outside the configured Chrome DevTools integration
- Any other browser tool, browser extension, browser bot, or remote browser automation mechanism

If browser interaction is needed, agents must use only the configured Chrome DevTools toolchain and must not fall back to alternatives.

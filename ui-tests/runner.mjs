// Real-browser scenario runner: replays the same JSON scenarios as
// src/scenario_lib (see ../scenarios/README.md for the format) through an
// actual Chromium tab via Playwright, so screens get genuinely rendered,
// clicked, and can be screenshotted rather than just asserted against raw
// HTTP responses.
//
// This is the optional, heavier layer described in scenarios/README.md.
// The fast/default layer is the Rust one (`cargo test`, `scenario-runner`);
// use this one when you specifically want visual/screenshot coverage or to
// exercise real DOM interaction.
//
// Usage:
//   node ui-tests/runner.mjs scenarios/*.json
//   GEONETIX_TEST_BASE_URL=http://staging.example.com node ui-tests/runner.mjs scenarios/login_existing_user.json
//
// Known limitation: Chromium needs real memory/CPU headroom (a few hundred
// MB+) to launch reliably; on very constrained hosts it can fail to start
// even with --no-sandbox. Prefer the Rust layer there.

import { chromium } from "playwright";
import { readFile, mkdir } from "node:fs/promises";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";

const SCRIPT_DIR = path.dirname(fileURLToPath(import.meta.url));
const SCREENSHOT_DIR = path.resolve(SCRIPT_DIR, "..", "artifacts", "site-screenshots");

function noSandboxEnabled() {
  return process.env.GEONETIX_BROWSER_NO_SANDBOX === "1";
}

function defaultLaunchArgs() {
  return noSandboxEnabled() ? ["--no-sandbox", "--disable-setuid-sandbox"] : [];
}

class TemplateContext {
  constructor(initialVars) {
    this.values = new Map(Object.entries(initialVars ?? {}));
  }

  set(key, value) {
    this.values.set(key, value);
  }

  render(input) {
    if (input == null) return input;
    return input.replace(/\{\{(.*?)\}\}/g, (_, rawToken) => {
      const token = rawToken.trim();
      if (this.values.has(token)) {
        return this.values.get(token);
      }
      if (token.startsWith("random.")) {
        const value = generateRandom(token.slice("random.".length));
        this.values.set(token, value);
        return value;
      }
      throw new Error(
        `unknown template variable {{${token}}} (not in scenario vars, not captured yet, and not a random.* generator)`,
      );
    });
  }
}

function generateRandom(kind) {
  const short = crypto.randomBytes(6).toString("hex");
  switch (kind) {
    case "full_name":
      return `Сценарий Тест ${short}`;
    case "email":
      return `scenario-${short}@example.com`;
    case "phone":
      return `+7999${Array.from(short.slice(0, 7)).map((c) => parseInt(c, 16) % 10).join("")}`;
    case "telegram":
      return `@scenario_${short}`;
    case "password":
      return `Scenario${short}!`;
    case "uuid":
      return crypto.randomUUID();
    default:
      throw new Error(`no random generator registered for random.${kind}`);
  }
}

function captureAfter(source, after) {
  if (!source) return null;
  const start = source.indexOf(after);
  if (start === -1) return null;
  const tail = source.slice(start + after.length);
  const match = tail.match(/^[A-Za-z0-9-]+/);
  return match ? match[0] : null;
}

async function visibleText(page) {
  const title = await page.title().catch(() => "");
  const bodyText = await page.locator("body").innerText().catch(() => "");
  return `${title} ${bodyText}`.split(/\s+/).filter(Boolean).join(" ");
}

/** Fills a form's visible fields and submits it, mirroring a real user
 * typing into inputs and clicking submit — rather than firing a raw POST
 * like the HTTP-level runner does.
 *
 * Some Geonetix pages (e.g. subscription actions) render several small
 * forms side by side, each just a single fixed `<input type="hidden">`
 * plus a submit button — effectively a set of distinct buttons rather
 * than one form to fill in. When a requested field is `hidden`, we treat
 * its value as "which button to click" and pick the form whose hidden
 * field already equals that value, instead of trying to overwrite it. */
async function fillAndSubmitForm(page, fields) {
  const fieldNames = Object.keys(fields);
  if (fieldNames.length === 0) {
    throw new Error("fill_form step has no fields");
  }

  const candidates = page.locator(`form:has(input[name="${fieldNames[0]}"])`);
  const candidateCount = await candidates.count();
  let form = candidates.first();
  for (let i = 0; i < candidateCount; i += 1) {
    const candidate = candidates.nth(i);
    let matchesAllHiddenFields = true;
    for (const [name, value] of Object.entries(fields)) {
      const input = candidate.locator(`[name="${name}"]`).first();
      if ((await input.count()) === 0) {
        matchesAllHiddenFields = false;
        break;
      }
      if ((await input.getAttribute("type")) === "hidden" && (await input.getAttribute("value")) !== value) {
        matchesAllHiddenFields = false;
        break;
      }
    }
    if (matchesAllHiddenFields) {
      form = candidate;
      break;
    }
  }

  for (const [name, value] of Object.entries(fields)) {
    const input = form.locator(`[name="${name}"]`).first();
    if ((await input.getAttribute("type")) === "hidden") {
      continue; // fixed value that identifies which button/form this is; nothing to type
    }
    const tagName = await input.evaluate((el) => el.tagName.toLowerCase());
    if (tagName === "select") {
      await input.selectOption(value);
    } else {
      await input.fill(value);
    }
  }

  const [response] = await Promise.all([
    page.waitForNavigation({ timeout: 20_000 }),
    form.evaluate((el) => (el.requestSubmit ? el.requestSubmit() : el.submit())),
  ]);
  return response;
}

async function runStep(page, ctx, step, screenshotBaseName) {
  const path_ = ctx.render(step.path ?? "");
  let response = null;

  switch (step.action) {
    case "navigate": {
      response = await page.goto(path_);
      break;
    }
    case "fill_form": {
      const currentUrl = page.url();
      if (!currentUrl || !currentUrl.includes(path_)) {
        await page.goto(path_);
      }
      const renderedFields = {};
      for (const [key, value] of Object.entries(step.fields ?? {})) {
        renderedFields[key] = ctx.render(value);
      }
      response = await fillAndSubmitForm(page, renderedFields);
      break;
    }
    case "wait_for_text": {
      const needle = ctx.render(step.text ?? "");
      const timeoutMs = step.timeout_ms ?? 8000;
      const pollMs = step.poll_ms ?? 400;
      const deadline = Date.now() + timeoutMs;
      let found = false;
      while (Date.now() < deadline) {
        response = await page.goto(path_);
        const text = await visibleText(page);
        if (text.includes(needle)) {
          found = true;
          break;
        }
        await new Promise((resolve) => setTimeout(resolve, pollMs));
      }
      if (!found) {
        throw new Error(`wait_for_text timed out after ${timeoutMs}ms waiting for "${needle}" on ${path_}`);
      }
      break;
    }
    default:
      throw new Error(`unknown step action \`${step.action}\``);
  }

  await applyCommonAssertions(page, ctx, step, response, path_);

  if (screenshotBaseName) {
    await mkdir(SCREENSHOT_DIR, { recursive: true });
    const file = path.join(SCREENSHOT_DIR, `${screenshotBaseName}.png`);
    await page.screenshot({ path: file, fullPage: true }).catch(() => {});
  }
}

async function applyCommonAssertions(page, ctx, step, response, renderedPath) {
  // Real browsers follow 3xx redirects transparently — a user never
  // observes the intermediate status, only where they land — so
  // `expect_status` in the 3xx range is meaningless here and is skipped;
  // `expect_redirect_contains` (checked below, against the final URL) is
  // this layer's equivalent of that assertion.
  if (step.expect_status != null && !(step.expect_status >= 300 && step.expect_status < 400)) {
    const status = response?.status();
    if (status !== step.expect_status) {
      throw new Error(`expected HTTP status ${step.expect_status}, got ${status} for ${renderedPath}`);
    }
  }

  if (step.expect_redirect_contains) {
    const fragment = ctx.render(step.expect_redirect_contains);
    if (!page.url().includes(fragment)) {
      throw new Error(`expected final URL to contain \`${fragment}\`, got \`${page.url()}\``);
    }
  }

  const text = await visibleText(page);
  for (const expected of step.expect_text ?? []) {
    const rendered = ctx.render(expected);
    if (!text.includes(rendered)) {
      throw new Error(`expected page text to contain \`${rendered}\`, it did not (path: ${renderedPath})`);
    }
  }

  for (const selector of step.expect_selector ?? []) {
    const count = await page.locator(selector).count();
    if (count === 0) {
      throw new Error(`expected an element matching selector \`${selector}\` on ${renderedPath}, found none`);
    }
  }

  for (const [name, after] of Object.entries(step.capture ?? {})) {
    const renderedAfter = ctx.render(after);
    const captured = captureAfter(page.url(), renderedAfter);
    if (!captured) {
      throw new Error(`capture \`${name}\`: could not find an id-like value after \`${renderedAfter}\` in current URL \`${page.url()}\``);
    }
    ctx.set(name, captured);
  }
}

async function runScenario(browser, baseUrl, scenarioPath, options) {
  const raw = await readFile(scenarioPath, "utf8");
  const scenario = JSON.parse(raw);
  const ctx = new TemplateContext(scenario.vars);
  const context = await browser.newContext({ baseURL: baseUrl });
  const page = await context.newPage();

  const started = Date.now();
  const stepReports = [];
  let passed = true;

  for (let index = 0; index < scenario.steps.length; index += 1) {
    const step = scenario.steps[index];
    const label = step.name ?? `step ${index + 1} (${step.action} ${step.path ?? ""})`;
    const stepStarted = Date.now();
    const screenshotBaseName = options.screenshots
      ? `${scenario.name}-${String(index + 1).padStart(2, "0")}-${(step.name ?? step.action).replace(/[^a-zA-Zа-яА-Я0-9]+/g, "_")}`
      : null;
    try {
      await runStep(page, ctx, step, screenshotBaseName);
      stepReports.push({ label, passed: true, durationMs: Date.now() - stepStarted, error: null });
    } catch (error) {
      stepReports.push({ label, passed: false, durationMs: Date.now() - stepStarted, error: error.message });
      passed = false;
      break;
    }
  }

  await context.close();
  return { scenarioName: scenario.name, steps: stepReports, passed, durationMs: Date.now() - started };
}

function printReport(report) {
  const status = report.passed ? "PASSED" : "FAILED";
  console.log(`[${status}] ${report.scenarioName} (${report.durationMs}ms)`);
  for (const step of report.steps) {
    const mark = step.passed ? "ok  " : "FAIL";
    console.log(`  ${mark} ${step.label} (${step.durationMs}ms)`);
    if (step.error) {
      console.log(`       -> ${step.error}`);
    }
  }
}

async function main() {
  const args = process.argv.slice(2);
  const screenshots = args.includes("--screenshots");
  const scenarioPaths = args.filter((arg) => arg !== "--screenshots");

  if (scenarioPaths.length === 0) {
    console.error("usage: node ui-tests/runner.mjs [--screenshots] <scenario.json> [more.json ...]");
    process.exit(1);
  }

  const baseUrl = process.env.GEONETIX_TEST_BASE_URL ?? "http://127.0.0.1:8080";
  const browser = await chromium.launch({ headless: true, args: defaultLaunchArgs() });

  let allPassed = true;
  try {
    for (const scenarioPath of scenarioPaths) {
      const report = await runScenario(browser, baseUrl, scenarioPath, { screenshots });
      printReport(report);
      allPassed &&= report.passed;
    }
  } finally {
    await browser.close();
  }

  process.exit(allPassed ? 0 : 1);
}

main().catch((error) => {
  console.error(`[ui-tests] fatal error: ${error.stack ?? error.message}`);
  process.exit(1);
});

import { chromium } from "playwright";

const targetUrl = process.argv[2];

if (!targetUrl) {
  console.error("Usage: node workers/yandex_browser_worker.mjs <yandex-card-url>");
  process.exit(1);
}

const REVIEW_LIMIT = 60;
const BROWSER_HEADERS = {
  "Accept-Language": "ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7",
  "Cache-Control": "no-cache",
  Pragma: "no-cache",
  Referer: "https://yandex.ru/maps/",
};

function collapseWhitespace(value) {
  return String(value ?? "").replace(/\s+/g, " ").trim();
}

function firstNonEmpty(...values) {
  for (const value of values.flat()) {
    const normalized = collapseWhitespace(value);
    if (normalized) {
      return normalized;
    }
  }
  return null;
}

function findNode(value, path) {
  if (!path.length) {
    return value;
  }

  if (Array.isArray(value)) {
    for (const item of value) {
      const match = findNode(item, path);
      if (match !== undefined) {
        return match;
      }
    }
    return undefined;
  }

  if (value && typeof value === "object") {
    const next = value[path[0]];
    if (next === undefined) {
      return undefined;
    }
    return findNode(next, path.slice(1));
  }

  return undefined;
}

function findText(value, path) {
  const node = findNode(value, path);
  if (typeof node === "string") {
    return collapseWhitespace(node);
  }
  if (typeof node === "number") {
    return String(node);
  }
  if (Array.isArray(node)) {
    for (const item of node) {
      const text = findText(item, []);
      if (text) {
        return text;
      }
    }
  }
  if (node && typeof node === "object" && !path.length && typeof node.name === "string") {
    return collapseWhitespace(node.name);
  }
  return null;
}

function findNumber(value, path) {
  const node = findNode(value, path);
  if (typeof node === "number") {
    return node;
  }
  if (typeof node === "string") {
    const parsed = Number(node.replace(",", "."));
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

function arrayLength(value, path) {
  const node = findNode(value, path);
  return Array.isArray(node) ? node.length : 0;
}

function photoCount(item) {
  const count = findNumber(item, ["photos", "count"]);
  if (count) {
    return Math.round(count);
  }
  return arrayLength(item, ["photos", "items"]);
}

function menuItemsCount(item) {
  const categories = findNode(item, ["topObjects", "categories"]);
  let count = 0;
  if (Array.isArray(categories)) {
    for (const category of categories) {
      if (Array.isArray(category?.categoryItems)) {
        count += category.categoryItems.length;
      }
    }
  }

  const features = findNode(item, ["features"]);
  if (Array.isArray(features)) {
    for (const feature of features) {
      const featureId = collapseWhitespace(feature?.id).toLowerCase();
      const featureName = collapseWhitespace(feature?.name).toLowerCase();
      if (
        featureId.includes("menu") ||
        featureId.includes("catalog") ||
        featureId.includes("price") ||
        featureName.includes("меню") ||
        featureName.includes("каталог") ||
        featureName.includes("прайс") ||
        featureName.includes("цена")
      ) {
        count += 1;
      }
    }
  }

  return Math.min(count, 50);
}

function enabledFeatureCount(item) {
  const features = findNode(item, ["features"]);
  if (!Array.isArray(features)) {
    return 0;
  }

  return Math.min(
    features.filter((feature) => {
      const value = feature?.value;
      if (typeof value === "boolean") {
        return value;
      }
      if (typeof value === "string") {
        return Boolean(collapseWhitespace(value));
      }
      if (typeof value === "number") {
        return true;
      }
      if (Array.isArray(value)) {
        return value.length > 0;
      }
      return false;
    }).length,
    20,
  );
}

function socialLinks(item) {
  const links = findNode(item, ["socialLinks"]);
  if (!Array.isArray(links)) {
    return [];
  }

  return links
    .map((link) => firstNonEmpty(link?.url, link?.href, link?.link))
    .filter(Boolean)
    .slice(0, 10);
}

function hasDelivery(item) {
  const features = findNode(item, ["features"]);
  if (!Array.isArray(features)) {
    return false;
  }

  return features.some((feature) => {
    const featureId = collapseWhitespace(feature?.id).toLowerCase();
    const featureName = collapseWhitespace(feature?.name).toLowerCase();
    return featureId.includes("delivery") || featureName.includes("доставка") || featureName.includes("delivery");
  });
}

function externalWebsiteUrl(item) {
  const candidates = [
    findText(item, ["businessLinks", "href"]),
    findText(item, ["sources", "href"]),
    findText(item, ["source", "href"]),
    findText(item, ["url"]),
  ].filter(Boolean);

  return (
    candidates.find((value) => {
      const lower = value.toLowerCase();
      return (
        (lower.startsWith("http://") || lower.startsWith("https://")) &&
        !lower.includes("yandex.") &&
        !lower.includes("ya.ru") &&
        !lower.includes("yastatic.net") &&
        !lower.includes("eda.yandex") &&
        !lower.includes("2gis.ru")
      );
    }) ?? null
  );
}

function sentimentFromAspects(item) {
  const aspects = findNode(item, ["aspects"]);
  if (!Array.isArray(aspects)) {
    return null;
  }

  let positive = 0;
  let neutral = 0;
  let negative = 0;
  for (const aspect of aspects) {
    positive += Number(aspect?.positive ?? 0);
    neutral += Number(aspect?.neutral ?? 0);
    negative += Number(aspect?.negative ?? 0);
  }

  const total = positive + neutral + negative;
  if (!total) {
    return null;
  }

  return {
    positive: Math.min(Math.max(Math.round((positive * 100) / total), 5), 95),
    negative: Math.min(Math.max(Math.round((negative * 100) / total), 0), 80),
  };
}

function sentimentFromReviews(reviews) {
  let positive = 0;
  let negative = 0;

  for (const review of reviews) {
    const rating = Number(review?.rating ?? 0);
    if (rating >= 4) {
      positive += 1;
    } else if (rating > 0 && rating <= 2) {
      negative += 1;
    }
  }

  const total = positive + negative;
  if (!total) {
    return null;
  }

  return {
    positive: Math.min(Math.max(Math.round((positive * 100) / total), 5), 95),
    negative: Math.min(Math.max(Math.round((negative * 100) / total), 0), 80),
  };
}

function findBusinessItem(state) {
  const items = findNode(state, ["stack", "results", "items"]);
  if (!Array.isArray(items)) {
    return null;
  }
  return items.find((item) => item?.type === "business") ?? null;
}

function extractStateFromHtml(html) {
  const scripts = [...html.matchAll(/<script[^>]*>([\s\S]*?)<\/script>/g)].map((match) =>
    match[1].trim(),
  );

  for (const script of scripts) {
    if (!script.startsWith("{")) {
      continue;
    }
    try {
      const parsed = JSON.parse(script);
      if (findBusinessItem(parsed)) {
        return parsed;
      }
    } catch {
      // ignore non-JSON script blocks
    }
  }

  return null;
}

function buildErrorResult(url, error) {
  return {
    ok: false,
    source_url: url,
    error: collapseWhitespace(error?.message ?? String(error)),
    diagnostics: ["Browser worker returned controlled error payload."],
  };
}

async function sleep(ms) {
  await new Promise((resolve) => setTimeout(resolve, ms));
}

function harvestReviewsFromPayload(payload, reviewsById) {
  const stack = [payload];

  while (stack.length) {
    const current = stack.pop();
    if (!current) {
      continue;
    }

    if (Array.isArray(current)) {
      for (const item of current) {
        stack.push(item);
      }
      continue;
    }

    if (typeof current !== "object") {
      continue;
    }

    if (Array.isArray(current.reviews)) {
      for (const review of current.reviews) {
        const reviewId = review?.reviewId ?? review?.id ?? review?.businessReviewId;
        if (!reviewId || reviewsById.size >= REVIEW_LIMIT) {
          continue;
        }
        reviewsById.set(String(reviewId), {
          reviewId: String(reviewId),
          text: collapseWhitespace(review?.text ?? review?.reviewText ?? ""),
          rating: Number(review?.rating ?? 0) || null,
        });
      }
    }

    for (const value of Object.values(current)) {
      stack.push(value);
    }
  }
}

async function maybeOpenReviewsTab(page) {
  const candidates = [
    page.getByRole("tab", { name: /отзыв/i }),
    page.getByRole("button", { name: /отзыв/i }),
    page.getByText(/все\s+\d+\s+отзыв/i).first(),
    page.getByText(/отзывы/i).first(),
  ];

  for (const locator of candidates) {
    try {
      if ((await locator.count()) > 0) {
        await locator.first().click({ timeout: 2500 });
        await page.waitForTimeout(1200);
        return true;
      }
    } catch {
      // try next candidate
    }
  }

  return false;
}

async function prepareScrollableReviewContainer(page) {
  return page.evaluate(() => {
    const nodes = [...document.querySelectorAll("div, section, aside")]
      .filter((node) => {
        const text = (node.innerText || "").toLowerCase();
        if (!text.includes("отзыв")) {
          return false;
        }
        const style = window.getComputedStyle(node);
        return /(auto|scroll)/.test(style.overflowY) && node.scrollHeight > node.clientHeight + 120;
      })
      .sort((left, right) => right.clientHeight - left.clientHeight);

    const target = nodes[0] || document.scrollingElement || document.documentElement;
    target.dataset.geonetixReviewScroller = "1";
    return true;
  });
}

async function scrollReviews(page) {
  await prepareScrollableReviewContainer(page);

  for (let step = 0; step < 18; step += 1) {
    await page.evaluate(() => {
      const target =
        document.querySelector('[data-geonetix-review-scroller="1"]') ||
        document.scrollingElement ||
        document.documentElement;
      target.scrollTop += 650;
    });
    await page.waitForTimeout(700);
  }
}

async function collectCardData(page, networkReviews) {
  const html = await page.content();
  const state = extractStateFromHtml(html);
  const item = state ? findBusinessItem(state) : null;

  if (!item) {
    throw new Error("Yandex card state was not found in rendered page");
  }

  const categoryHint =
    firstNonEmpty(
      findText(item, ["categories", "name"]),
      findText(item, ["topObjects", "rubric", "name"]),
      "Яндекс Карты",
    ) ?? "Яндекс Карты";
  const workingTime = findText(item, ["workingTimeText"]);
  const currentStatus = findText(item, ["currentWorkingStatus", "text"]);
  const additionalAddress = findText(item, ["additionalAddress"]);
  const title =
    firstNonEmpty(findText(item, ["title"]), findText(item, ["shortTitle"]), "Профиль") ?? "Профиль";
  const address =
    firstNonEmpty(
      findText(item, ["fullAddress"]),
      findText(item, ["address"]),
      findText(item, ["description"]),
      "локальный кластер Яндекс Карт",
    ) ?? "локальный кластер Яндекс Карт";
  const description =
    firstNonEmpty([categoryHint, currentStatus, workingTime, additionalAddress].join(". "), address) ??
    address;
  const rating =
    findNumber(item, ["ratingData", "ratingValue"]) ??
    findNumber(item, ["ratingData", "rating"]) ??
    null;
  const reviewCount =
    Math.round(
      findNumber(item, ["ratingData", "reviewCount"]) ??
        findNumber(item, ["ratingData", "ratingCount"]) ??
        networkReviews.size,
    ) || null;
  const websiteUrl = externalWebsiteUrl(item);
  const aspectSentiment = sentimentFromAspects(item);
  const networkSentiment = sentimentFromReviews([...networkReviews.values()]);
  const sentiment = aspectSentiment ?? networkSentiment ?? { positive: 50, negative: 10 };

  return {
    final_url: page.url(),
    card_title: title,
    address,
    description,
    rating,
    review_count: reviewCount,
    photo_count: Math.min(photoCount(item), 200),
    menu_items_count: Math.min(menuItemsCount(item), 50),
    contact_count: Math.min(
      arrayLength(item, ["phones"]) + arrayLength(item, ["businessLinks"]) + (websiteUrl ? 1 : 0),
      12,
    ),
    social_count: Math.min(arrayLength(item, ["socialLinks"]), 10),
    social_links: socialLinks(item),
    feature_count: Math.min(enabledFeatureCount(item), 20),
    has_delivery: hasDelivery(item),
    positive_review_share: sentiment.positive,
    negative_review_share: sentiment.negative,
    category_hint: categoryHint,
    website_url: websiteUrl,
    diagnostics: [
      `Browser worker rendered page and extracted embedded Yandex state.`,
      `Captured review records from network: ${networkReviews.size}.`,
      workingTime ? `Рабочее время: ${workingTime}` : null,
    ].filter(Boolean),
  };
}

function noSandboxEnabled() {
  return process.env.GEONETIX_BROWSER_NO_SANDBOX === "1";
}

function defaultLaunchArgs() {
  return noSandboxEnabled() ? ["--no-sandbox", "--disable-setuid-sandbox"] : [];
}

export async function parseYandexCard(url, options = {}) {
  const {
    launchBrowser = () => chromium.launch({ headless: true, args: defaultLaunchArgs() }),
    logger = console,
    delayMs = 1200,
  } = options;

  let browser;
  let context;

  try {
    browser = await launchBrowser();
    context = await browser.newContext({
      locale: "ru-RU",
      timezoneId: "Europe/Moscow",
      viewport: { width: 1440, height: 1080 },
      userAgent:
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36",
      extraHTTPHeaders: BROWSER_HEADERS,
    });

    const page = await context.newPage();
    page.setDefaultTimeout(15000);

    const networkReviews = new Map();
    page.on("response", async (response) => {
      try {
        const headers = await response.allHeaders();
        const contentType = headers["content-type"] || "";
        if (!contentType.includes("application/json")) {
          return;
        }
        if (!response.url().includes("yandex")) {
          return;
        }

        const payload = await response.json();
        harvestReviewsFromPayload(payload, networkReviews);
      } catch {
        // ignore non-JSON or blocked responses
      }
    });

    await page.goto(url, { waitUntil: "domcontentloaded" });
    await page.waitForLoadState("networkidle", { timeout: 10000 }).catch(() => {});
    await sleep(delayMs);

    await maybeOpenReviewsTab(page);
    await sleep(700);
    await scrollReviews(page);
    await sleep(700);

    return {
      ok: true,
      ...(await collectCardData(page, networkReviews)),
    };
  } catch (error) {
    logger.error(`[yandex-browser-worker] ${collapseWhitespace(error?.message ?? String(error))}`);
    return buildErrorResult(url, error);
  } finally {
    if (context) {
      await context.close().catch(() => {});
    }
    if (browser) {
      await browser.close().catch(() => {});
    }
  }
}

const result = await parseYandexCard(targetUrl);
console.log(JSON.stringify(result));

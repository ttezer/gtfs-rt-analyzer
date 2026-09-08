import { readFile, writeFile } from "node:fs/promises";

const catalogPath = new URL("../ui/feeds.json", import.meta.url);
const sourceUrl = process.env.MOBILITYDATABASE_CATALOG_URL
  || "https://files.mobilitydatabase.org/feeds_v2.csv";
const previous = await readJson(catalogPath, { feeds: [] });
const response = await fetch(sourceUrl, { redirect: "follow" });
if (!response.ok) throw new Error(`MobilityDatabase CSV ${response.status} ${response.statusText}`);

const rows = parseCsv(await response.text());
const feeds = rows
  .filter(isCandidate)
  .map(toFeed)
  .sort((a, b) => a.id.localeCompare(b.id));
const today = new Date().toISOString().slice(0, 10);
const unchanged = JSON.stringify(previous.feeds || []) === JSON.stringify(feeds);
const output = {
  source: "MobilityDatabase",
  source_url: sourceUrl,
  updated_at: unchanged ? (previous.updated_at || today) : today,
  policy: "GTFS-RT, no authentication, HTTPS, no credential-like query parameter",
  feeds,
};
const serialized = `${JSON.stringify(output)}\n`;
const oldSerialized = await readText(catalogPath);

if (oldSerialized !== serialized) {
  await writeFile(catalogPath, serialized);
  console.log(`Wrote ${feeds.length} catalog feeds to ${catalogPath.pathname}`);
} else {
  console.log(`Catalog unchanged (${feeds.length} feeds)`);
}

function isCandidate(row) {
  if (row.data_type !== "gtfs_rt") return false;
  if (!["active", "deprecated", "development"].includes(row.status)) return false;
  if ((row["urls.authentication_type"] || "").trim() !== "0") return false;

  const url = row["urls.latest"] || row["urls.direct_download"] || "";
  if (!url.startsWith("https://")) return false;
  if (hasCredentialQuery(url)) return false;
  return Boolean(row.id && row.entity_type);
}

function toFeed(row) {
  return {
    id: row.id,
    provider: row.provider || null,
    name: row.name || null,
    entity_types: row.entity_type.split("|").filter(Boolean).map(entityType),
    official: parseBoolean(row.is_official),
    status: row.status,
    url: row["urls.latest"] || row["urls.direct_download"],
    static_reference: row.static_reference || null,
    country_code: row["location.country_code"] || null,
    municipality: row["location.municipality"] || null,
    center: boundingBoxCenter(row),
  };
}

function entityType(value) {
  return {
    vp: "vehicle_positions",
    tu: "trip_updates",
    sa: "alerts",
  }[value] || value;
}

function parseBoolean(value) {
  if (value === "True") return true;
  if (value === "False") return false;
  return null;
}

function boundingBoxCenter(row) {
  const values = [
    row["location.bounding_box.minimum_latitude"],
    row["location.bounding_box.maximum_latitude"],
    row["location.bounding_box.minimum_longitude"],
    row["location.bounding_box.maximum_longitude"],
  ];
  if (values.some((value) => !value?.trim())) return null;
  const [minLat, maxLat, minLon, maxLon] = values.map(Number);
  if (![minLat, maxLat, minLon, maxLon].every(Number.isFinite)) return null;
  return {
    lat: Number(((minLat + maxLat) / 2).toFixed(6)),
    lon: Number(((minLon + maxLon) / 2).toFixed(6)),
  };
}

function hasCredentialQuery(value) {
  const credentialNames = new Set([
    "access_token",
    "access-token",
    "api_key",
    "api-key",
    "apikey",
    "auth",
    "authorization",
    "client_secret",
    "client-secret",
    "key",
    "password",
    "secret",
    "sig",
    "signature",
    "subscription_key",
    "subscription-key",
    "token",
  ]);
  try {
    const url = new URL(value);
    return [...url.searchParams.keys()].some((name) => credentialNames.has(name.toLowerCase()));
  } catch {
    return true;
  }
}

function parseCsv(text) {
  const rows = [];
  let row = [];
  let field = "";
  let quoted = false;
  const input = text.replace(/^\uFEFF/, "");

  for (let index = 0; index < input.length; index += 1) {
    const character = input[index];
    if (quoted) {
      if (character === '"' && input[index + 1] === '"') {
        field += '"';
        index += 1;
      } else if (character === '"') {
        quoted = false;
      } else {
        field += character;
      }
    } else if (character === '"') {
      quoted = true;
    } else if (character === ",") {
      row.push(field);
      field = "";
    } else if (character === "\n") {
      row.push(field);
      rows.push(row);
      row = [];
      field = "";
    } else if (character !== "\r") {
      field += character;
    }
  }

  if (field || row.length) {
    row.push(field);
    rows.push(row);
  }
  const headers = rows.shift() || [];
  return rows
    .filter((candidate) => candidate.length === headers.length)
    .map((candidate) => Object.fromEntries(headers.map((header, index) => [header, candidate[index]])));
}

async function readJson(url, fallback) {
  try {
    return JSON.parse(await readFile(url, "utf8"));
  } catch {
    return fallback;
  }
}

async function readText(url) {
  try {
    return await readFile(url, "utf8");
  } catch {
    return null;
  }
}

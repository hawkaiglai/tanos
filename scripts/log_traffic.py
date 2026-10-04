#!/usr/bin/env python3
"""Keep a long-term log of a GitHub repo's traffic.

GitHub only shows the last 14 days of clones and views. This script reads them
from the REST API, merges them into traffic.csv (a day that shows up in two runs
is stored once), saves the top referrers and pages, and rebuilds README.md with
a week-by-week report.

    python log_traffic.py --repo OWNER/REPO --data-dir ./traffic-data
    python log_traffic.py --data-dir ./traffic-data --report-only

The token is read from TRAFFIC_TOKEN (or GH_TOKEN / GITHUB_TOKEN). It needs
"Administration: Read-only" on the repo for a fine-grained token, or the "repo"
scope for a classic one. Standard library only.
"""
import argparse
import csv
import json
import os
import sys
import urllib.error
import urllib.request
from datetime import date, datetime, timedelta, timezone

API = os.environ.get("GITHUB_API_URL", "https://api.github.com").rstrip("/")
WINDOW_DAYS = 14  # how far back GitHub's traffic numbers reach
DAILY_FIELDS = ["date", "clones", "unique_cloners", "views", "unique_visitors", "source"]
BLOCKS = "▁▂▃▄▅▆▇█"

HINTS = {
    401: "GitHub rejected the token. Check the TRAFFIC_TOKEN secret.",
    403: "The token can't read traffic. It needs 'Administration: Read-only' on this repo "
         "(fine-grained token) or the 'repo' scope (classic token).",
    404: "Repo not found, or the token has no access to it. Check the repo name and the "
         "token's repository access.",
}


def utc_today():
    return datetime.now(timezone.utc).date()


# --- GitHub API ------------------------------------------------------------

def api_get(repo, endpoint, token):
    req = urllib.request.Request(
        f"{API}/repos/{repo}/traffic/{endpoint}",
        headers={
            "Authorization": f"Bearer {token}",
            "Accept": "application/vnd.github+json",
            "User-Agent": "traffic-log",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.load(resp)
    except urllib.error.HTTPError as err:
        sys.exit(f"GET traffic/{endpoint} failed with HTTP {err.code}. {HINTS.get(err.code, '')}".strip())
    except urllib.error.URLError as err:
        sys.exit(f"GET traffic/{endpoint} failed: {err.reason}")


def collect_days(clones, views, today):
    """Combine the clones and views answers into
    {date: (clones, unique_cloners, views, unique_visitors)}.

    Today is skipped because it is still in progress; a later run stores it
    once it is complete.
    """
    def by_day(entries):
        return {date.fromisoformat(e["timestamp"][:10]): (e["count"], e["uniques"]) for e in entries}

    c = by_day(clones.get("clones", []))
    v = by_day(views.get("views", []))
    out = {}
    for d in sorted(set(c) | set(v)):
        if d < today:
            out[d] = c.get(d, (0, 0)) + v.get(d, (0, 0))
    return out


# --- CSV storage -----------------------------------------------------------

def read_csv(path):
    if not os.path.exists(path):
        return []
    with open(path, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f))


def write_csv(path, fields, rows):
    with open(path, "w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def load_daily(path):
    """{date: (clones, unique_cloners, views, unique_visitors, source)}"""
    store = {}
    for r in read_csv(path):
        store[date.fromisoformat(r["date"])] = (
            int(r["clones"]), int(r["unique_cloners"]),
            int(r["views"]), int(r["unique_visitors"]),
            r.get("source") or "api",
        )
    return store


def save_daily(path, store):
    rows = [dict(zip(DAILY_FIELDS, (d.isoformat(),) + store[d])) for d in sorted(store)]
    write_csv(path, DAILY_FIELDS, rows)


def merge_daily(store, fetched, today):
    """Fold freshly fetched days into the store. Returns (added, updated)."""
    added = updated = 0
    for d, vals in fetched.items():
        row = vals + ("api",)
        if d not in store:
            added += 1
        elif store[d] != row:
            updated += 1
        store[d] = row  # the API is authoritative for any day it returns

    # GitHub may leave out days with no traffic. A day inside the window that we
    # have never recorded was therefore a zero day. Recorded days are never
    # overwritten by a zero here.
    for i in range(1, WINDOW_DAYS + 1):
        d = today - timedelta(days=i)
        if d not in store:
            store[d] = (0, 0, 0, 0, "api")
            added += 1
    return added, updated


def write_snapshot(path, fields, today, rows):
    """Add today's top-10 list; running twice on the same day replaces it."""
    keep = [r for r in read_csv(path) if r["snapshot_date"] != today.isoformat()]
    new = [{"snapshot_date": today.isoformat(), **r} for r in rows]
    write_csv(path, fields, keep + new)


# --- Report ----------------------------------------------------------------

def weekly(store):
    """{monday: [days logged, clones, unique cloners, views, unique visitors]},
    including weeks with no data so gaps stay visible."""
    weeks = {}
    for d, (c, uc, v, uv, _source) in store.items():
        monday = d - timedelta(days=d.weekday())
        w = weeks.setdefault(monday, [0, 0, 0, 0, 0])
        w[0] += 1
        w[1] += c
        w[2] += uc
        w[3] += v
        w[4] += uv
    if weeks:
        monday, last = min(weeks), max(weeks)
        while monday <= last:
            weeks.setdefault(monday, [0, 0, 0, 0, 0])
            monday += timedelta(days=7)
    return dict(sorted(weeks.items()))


def spark(values):
    """One block per value, scaled to the largest. None (no data) is a dot."""
    have = [v for v in values if v is not None]
    top = max(have) if have else 0
    chars = []
    for v in values:
        if v is None:
            chars.append("·")
        elif v == 0 or top == 0:
            chars.append(BLOCKS[0])
        else:
            chars.append(BLOCKS[max(1, round(v / top * 7))])
    return "".join(chars)


def gaps(days):
    """Runs of dates with no data between the first and last logged day."""
    return [(a + timedelta(days=1), b - timedelta(days=1))
            for a, b in zip(days, days[1:]) if (b - a).days > 1]


def build_report(store, repo, today):
    lines = [f"# Traffic log: {repo}", ""]
    if not store:
        return "\n".join(lines + ["No data yet.", ""])

    days = sorted(store)
    weeks = weekly(store)

    status = f"Updated {today.isoformat()}. {len(days)} days logged, {days[0].isoformat()} to {days[-1].isoformat()}."
    missing = gaps(days)
    if missing:
        spans = [a.isoformat() if a == b else f"{a.isoformat()} to {b.isoformat()}" for a, b in missing]
        status += " Not logged: " + ", ".join(spans) + "."
    typed = sum(1 for d in days if store[d][4] == "screenshot")
    if typed:
        status += f" {typed} early days were copied from screenshots (see the source column)."
    lines += [status, ""]

    clone_rate = [w[1] / w[0] if w[0] else None for w in weeks.values()]
    view_rate = [w[3] / w[0] if w[0] else None for w in weeks.values()]
    lines += [
        "## Trend",
        "",
        f"One character per week, oldest first, starting with the week of {next(iter(weeks))}. "
        "Height is the average per logged day, scaled to the busiest week in each row; "
        "a dot means no data.",
        "",
        "```",
        f"Clones  {spark(clone_rate)}",
        f"Views   {spark(view_rate)}",
        "```",
        "",
    ]

    end = days[-1]
    for label, hi in (("Last 28 days", end), ("Previous 28 days", end - timedelta(days=28))):
        lo = hi - timedelta(days=27)
        window = [d for d in days if lo <= d <= hi]
        span = f"{lo.isoformat()} to {hi.isoformat()}"
        if window:
            c = sum(store[d][0] for d in window)
            v = sum(store[d][2] for d in window)
            n = len(window)
            lines.append(f"- {label} ({span}): {c} clones ({c / n:.2f}/day), "
                         f"{v} views ({v / n:.2f}/day), {n} of 28 days logged")
        else:
            lines.append(f"- {label} ({span}): no data")
    total_c = sum(store[d][0] for d in days)
    total_v = sum(store[d][2] for d in days)
    lines.append(f"- All logged days: {total_c} clones, {total_v} views over {len(days)} days")

    lines += [
        "",
        "## Weekly totals",
        "",
        "| Week of | Days logged | Clones | Cloners† | Views | Visitors† |",
        "|---|---|---|---|---|---|",
    ]
    for monday, (n, c, uc, v, uv) in reversed(list(weeks.items())):
        cells = [str(x) for x in (c, uc, v, uv)] if n else ["n/a"] * 4
        lines.append(f"| {monday.isoformat()} | {n}/7 | " + " | ".join(cells) + " |")
    lines += [
        "",
        "† Sum of each day's unique count, so someone who shows up on two days counts twice.",
        "",
        "Raw data: `traffic.csv` has one row per day. `referrers.csv` and `paths.csv` hold the "
        "top-10 lists GitHub reported on each run; each covers the 14 days before that run, so "
        "snapshots overlap.",
        "",
    ]
    return "\n".join(lines)


# --- Entry point -----------------------------------------------------------

def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--repo", default=os.environ.get("GITHUB_REPOSITORY"),
                    help="OWNER/REPO (default: $GITHUB_REPOSITORY)")
    ap.add_argument("--data-dir", default=".",
                    help="folder for traffic.csv, referrers.csv, paths.csv and README.md")
    ap.add_argument("--report-only", action="store_true",
                    help="rebuild README.md from the CSVs without calling the API")
    args = ap.parse_args()

    def out(name):
        return os.path.join(args.data_dir, name)

    os.makedirs(args.data_dir, exist_ok=True)
    today = utc_today()
    store = load_daily(out("traffic.csv"))

    if not args.report_only:
        token = os.environ.get("TRAFFIC_TOKEN") or os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
        if not args.repo:
            sys.exit("Pass --repo OWNER/REPO (or set GITHUB_REPOSITORY).")
        if not token:
            sys.exit("No token found. Add a repository secret named TRAFFIC_TOKEN "
                     "(setup steps are in the comments of traffic-log.yml).")

        # Fetch everything before writing anything, so a failure leaves the data untouched.
        clones = api_get(args.repo, "clones", token)
        views = api_get(args.repo, "views", token)
        referrers = api_get(args.repo, "popular/referrers", token)
        paths = api_get(args.repo, "popular/paths", token)

        added, updated = merge_daily(store, collect_days(clones, views, today), today)
        save_daily(out("traffic.csv"), store)
        write_snapshot(out("referrers.csv"), ["snapshot_date", "referrer", "views", "uniques"], today,
                       [{"referrer": r["referrer"], "views": r["count"], "uniques": r["uniques"]}
                        for r in referrers])
        write_snapshot(out("paths.csv"), ["snapshot_date", "path", "views", "uniques"], today,
                       [{"path": p["path"], "views": p["count"], "uniques": p["uniques"]}
                        for p in paths])
        print(f"{args.repo}: {added} days added, {updated} updated, {len(store)} stored.")

    report = build_report(store, args.repo or "this repo", today)
    with open(out("README.md"), "w", encoding="utf-8") as f:
        f.write(report)
    print(report)


if __name__ == "__main__":
    main()

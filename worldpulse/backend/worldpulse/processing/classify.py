"""Rule-based topic classification (transparent and testable).

Each category has weighted keyword patterns. The article gets the category
with the highest score; scores are kept so the dashboard can show why. An
article that matches nothing is "general" — we do not guess.
"""
from __future__ import annotations

import re

CATEGORIES = [
    "geopolitics", "diplomacy", "military_security", "elections", "economics", "financial_markets",
    "energy", "trade", "natural_disasters", "technology", "cybersecurity", "public_health", "environment",
]

LABELS = {
    "geopolitics": "Geopolitics", "diplomacy": "Diplomacy", "military_security": "Military & security",
    "elections": "Elections", "economics": "Economics", "financial_markets": "Financial markets",
    "energy": "Energy", "trade": "Trade", "natural_disasters": "Natural disasters", "technology": "Technology",
    "cybersecurity": "Cybersecurity", "public_health": "Public health", "environment": "Environment",
    "general": "General",
}

_RULES: dict[str, list[tuple[str, float]]] = {
    "geopolitics": [(r"sanction\w*", 2), (r"geopolitic\w*", 3), (r"sovereignty", 2), (r"annex\w*", 2), (r"territor(y|ial) dispute", 3),
                    (r"coup", 3), (r"regime", 1.5), (r"protest\w*", 1.5), (r"unrest", 2), (r"opposition leader", 1.5)],
    "diplomacy": [(r"diplomat\w*", 3), (r"summit", 2.5), (r"foreign minister", 3), (r"ambassador", 2.5), (r"bilateral", 2.5),
                  (r"talks", 1.5), (r"treaty", 2.5), (r"ceasefire", 2), (r"peace (deal|talks|plan|agreement)", 3), (r"united nations|\bun\b security council", 2),
                  (r"envoy", 2.5), (r"state visit", 3), (r"embassy", 2)],
    "military_security": [(r"military", 2.5), (r"troops?", 2.5), (r"airstrikes?|air strikes?", 3), (r"missiles?", 3), (r"drones? (attack|strike)", 3),
                          (r"\bwar\b", 2), (r"shelling", 3), (r"armed forces|army|navy", 2), (r"militant\w*|insurgen\w*", 2.5),
                          (r"terror\w*", 2.5), (r"killed in (an )?attack|bomb(ing)?s?", 2.5), (r"border clash\w*", 3), (r"defen[cs]e ministry", 2), (r"hostages?", 2)],
    "elections": [(r"election\w*", 3), (r"\bvot(e|es|ers|ing)\b", 2.5), (r"ballot\w*", 3), (r"polls? (show|open|close)", 2.5), (r"referendum", 3),
                  (r"candidate\w*", 2), (r"campaign trail", 2), (r"electoral", 3), (r"inaugurat\w*", 2)],
    "economics": [(r"\bgdp\b", 3), (r"inflation", 3), (r"recession", 3), (r"central bank", 3), (r"interest rates?", 2.5), (r"unemployment|jobless", 2.5),
                  (r"econom(y|ic|ies)", 2), (r"budget deficit|fiscal", 2.5), (r"\bimf\b|world bank", 2.5), (r"debt", 1.5), (r"rate (cut|hike)", 3), (r"consumer prices", 2.5)],
    "financial_markets": [(r"stocks?\b", 2.5), (r"shares", 2), (r"wall street", 3), (r"s&p 500|nasdaq|dow jones|ftse|nikkei|sensex|nifty|dax|hang seng", 3),
                          (r"bond yields?|treasur(y|ies)", 2.5), (r"bitcoin|crypto\w*", 2.5), (r"\bipo\b", 2.5), (r"market (rally|selloff|sell-off|rout)", 3),
                          (r"investors?", 1.5), (r"currenc(y|ies)|\byuan\b|\brupee\b|\byen\b|\beuro\b|dollar", 1.5), (r"gold prices?", 2.5)],
    "energy": [(r"\boil\b", 2.5), (r"crude", 3), (r"\bopec\+?", 3), (r"natural gas|\blng\b", 3), (r"pipeline", 2), (r"refiner(y|ies)", 2.5),
               (r"power (grid|outage|plant)", 2.5), (r"electricity", 2), (r"nuclear (plant|power|reactor)", 2.5), (r"renewable|solar|wind power", 2), (r"energy", 1.5)],
    "trade": [(r"tariffs?", 3), (r"\btrade\b", 2), (r"exports?|imports?", 2), (r"\bwto\b", 3), (r"supply chains?", 2.5), (r"shipping|freight|cargo", 2),
              (r"trade (deal|war|agreement|deficit|surplus)", 3), (r"embargo", 2.5), (r"customs", 1.5), (r"red sea|strait of hormuz|suez|panama canal", 2)],
    "natural_disasters": [(r"earthquakes?|quake|tremor", 3.5), (r"tsunami", 3.5), (r"hurricanes?|typhoons?|cyclones?", 3.5), (r"floods?|flooding", 3),
                          (r"wildfires?|bushfires?|forest fires?", 3), (r"volcan\w*|eruption", 3.5), (r"landslides?|mudslides?", 3), (r"drought", 2.5),
                          (r"tornado\w*", 3.5), (r"heatwave|heat wave", 2.5), (r"storm\b", 1.5), (r"evacuat\w*", 1.5)],
    "technology": [(r"artificial intelligence|\bai\b", 2.5), (r"semiconductor\w*|chips?\b", 2.5), (r"tech (giant|company|firm)s?", 2), (r"software", 1.5),
                   (r"smartphone\w*", 2), (r"satellite\w*", 1.5), (r"quantum", 2), (r"startup\w*", 1.5), (r"apple|google|microsoft|nvidia|meta|openai|tesla|samsung|tsmc", 1.5)],
    "cybersecurity": [(r"cyber\s?attack\w*|cyber-attack\w*", 3.5), (r"ransomware", 3.5), (r"hack(ed|ers?|ing)", 3), (r"data breach", 3.5), (r"malware", 3.5),
                      (r"vulnerabilit(y|ies)|cve-\d{4}", 3), (r"phishing", 3), (r"ddos", 3.5), (r"cybersecurity|cyber security", 3), (r"espionage", 1.5)],
    "public_health": [(r"outbreak", 3), (r"epidemic|pandemic", 3.5), (r"virus|viral infection", 2.5), (r"cholera|measles|ebola|mpox|dengue|malaria|polio|bird flu|h5n1", 3.5),
                      (r"\bwho\b|world health organization", 2.5), (r"vaccin\w*", 2.5), (r"hospital\w*", 1.5), (r"disease", 2), (r"public health", 3), (r"famine|malnutrition", 2.5)],
    "environment": [(r"climate", 3), (r"emissions?", 2.5), (r"carbon", 2), (r"pollution", 3), (r"deforestation", 3), (r"biodiversity", 3), (r"\bcop\d{2}\b", 3),
                    (r"glacier\w*|sea level", 2.5), (r"global warming", 3), (r"environment\w*", 2), (r"air quality", 2.5)],
}

_COMPILED = {cat: [(re.compile(r"\b" + p + r"\b" if not p.startswith("\\b") else p, re.I), w) for p, w in rules] for cat, rules in _RULES.items()}

# A source configured with a category (e.g. CISA advisories) adds weight.
SOURCE_PRIOR = 2.0


def classify(text: str, source_category: str | None = None) -> tuple[str, dict[str, float]]:
    scores: dict[str, float] = {}
    for cat, rules in _COMPILED.items():
        s = sum(w for rx, w in rules if rx.search(text))
        if s:
            scores[cat] = round(s, 2)
    if source_category in _COMPILED:
        scores[source_category] = round(scores.get(source_category, 0) + SOURCE_PRIOR, 2)
    if not scores:
        return "general", {}
    best = max(scores, key=scores.get)
    if scores[best] < 2:
        return "general", scores
    return best, scores

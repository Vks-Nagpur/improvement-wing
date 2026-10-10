"""Country gazetteer used to associate text with ISO 3166-1 countries.

This is reference data (names, common aliases, capitals and demonyms), not
statistics. Population, capital coordinates, income level and indicators are
fetched from the World Bank at run time so they carry a source and a date.

Line format: ISO2|ISO3|Name|Region|aliases separated by ';'
An alias starting with '~' is ambiguous (it is also a person, a US state, a
common word...). A match on an ambiguous alias alone gives low confidence.
Matching is case-sensitive and on word boundaries.
"""
from __future__ import annotations

import re
from dataclasses import dataclass, field

_DATA = """
AF|AFG|Afghanistan|South Asia|Afghan;Afghans;Kabul;Taliban
AL|ALB|Albania|Europe|Albanian;Tirana
DZ|DZA|Algeria|Middle East & North Africa|Algerian;Algiers
AD|AND|Andorra|Europe|Andorran
AO|AGO|Angola|Sub-Saharan Africa|Angolan;Luanda
AG|ATG|Antigua and Barbuda|Latin America & Caribbean|Antigua
AR|ARG|Argentina|Latin America & Caribbean|Argentine;Argentinian;Buenos Aires;Milei
AM|ARM|Armenia|Europe & Central Asia|Armenian;Yerevan
AU|AUS|Australia|East Asia & Pacific|Australian;Australians;Canberra;Sydney;Melbourne
AT|AUT|Austria|Europe|Austrian;Vienna
AZ|AZE|Azerbaijan|Europe & Central Asia|Azerbaijani;Baku
BS|BHS|Bahamas|Latin America & Caribbean|Bahamian;Nassau
BH|BHR|Bahrain|Middle East & North Africa|Bahraini;Manama
BD|BGD|Bangladesh|South Asia|Bangladeshi;Dhaka
BB|BRB|Barbados|Latin America & Caribbean|Barbadian;Bridgetown
BY|BLR|Belarus|Europe|Belarusian;Minsk;Lukashenko
BE|BEL|Belgium|Europe|Belgian;Brussels
BZ|BLZ|Belize|Latin America & Caribbean|Belizean
BJ|BEN|Benin|Sub-Saharan Africa|Beninese;Porto-Novo;Cotonou
BT|BTN|Bhutan|South Asia|Bhutanese;Thimphu
BO|BOL|Bolivia|Latin America & Caribbean|Bolivian;La Paz
BA|BIH|Bosnia and Herzegovina|Europe|Bosnia;Bosnian;Sarajevo
BW|BWA|Botswana|Sub-Saharan Africa|Gaborone
BR|BRA|Brazil|Latin America & Caribbean|Brazilian;Brasilia;Brasília;Sao Paulo;São Paulo;Rio de Janeiro;Lula
BN|BRN|Brunei|East Asia & Pacific|Bruneian
BG|BGR|Bulgaria|Europe|Bulgarian;Sofia
BF|BFA|Burkina Faso|Sub-Saharan Africa|Burkinabe;Ouagadougou
BI|BDI|Burundi|Sub-Saharan Africa|Burundian;Gitega
CV|CPV|Cabo Verde|Sub-Saharan Africa|Cape Verde
KH|KHM|Cambodia|East Asia & Pacific|Cambodian;Phnom Penh
CM|CMR|Cameroon|Sub-Saharan Africa|Cameroonian;Yaounde;Yaoundé
CA|CAN|Canada|North America|Canadian;Canadians;Ottawa;Toronto;Montreal;Vancouver
CF|CAF|Central African Republic|Sub-Saharan Africa|Bangui
TD|TCD|Chad|Sub-Saharan Africa|Chadian;N'Djamena
CL|CHL|Chile|Latin America & Caribbean|Chilean;Santiago
CN|CHN|China|East Asia & Pacific|Chinese;Beijing;Shanghai;Xi Jinping;Hong Kong;Shenzhen
CO|COL|Colombia|Latin America & Caribbean|Colombian;Bogota;Bogotá
KM|COM|Comoros|Sub-Saharan Africa|Comorian
CD|COD|Democratic Republic of the Congo|Sub-Saharan Africa|DR Congo;DRC;Democratic Republic of Congo;Kinshasa;Goma;~Congo;~Congolese
CG|COG|Republic of the Congo|Sub-Saharan Africa|Republic of Congo;Congo-Brazzaville;Brazzaville
CR|CRI|Costa Rica|Latin America & Caribbean|Costa Rican;San Jose
CI|CIV|Cote d'Ivoire|Sub-Saharan Africa|Côte d'Ivoire;Ivory Coast;Ivorian;Abidjan;Yamoussoukro
HR|HRV|Croatia|Europe|Croatian;Zagreb
CU|CUB|Cuba|Latin America & Caribbean|Cuban;Havana
CY|CYP|Cyprus|Europe|Cypriot;Nicosia
CZ|CZE|Czechia|Europe|Czech Republic;Czech;Prague
DK|DNK|Denmark|Europe|Danish;Copenhagen
DJ|DJI|Djibouti|Sub-Saharan Africa|Djiboutian
DM|DMA|Dominica|Latin America & Caribbean|~Dominica
DO|DOM|Dominican Republic|Latin America & Caribbean|Santo Domingo
EC|ECU|Ecuador|Latin America & Caribbean|Ecuadorian;Quito;Guayaquil
EG|EGY|Egypt|Middle East & North Africa|Egyptian;Cairo;Suez Canal
SV|SLV|El Salvador|Latin America & Caribbean|Salvadoran;San Salvador;Bukele
GQ|GNQ|Equatorial Guinea|Sub-Saharan Africa|Malabo
ER|ERI|Eritrea|Sub-Saharan Africa|Eritrean;Asmara
EE|EST|Estonia|Europe|Estonian;Tallinn
SZ|SWZ|Eswatini|Sub-Saharan Africa|Swaziland;Mbabane
ET|ETH|Ethiopia|Sub-Saharan Africa|Ethiopian;Addis Ababa;Tigray
FJ|FJI|Fiji|East Asia & Pacific|Fijian;Suva
FI|FIN|Finland|Europe|Finnish;Helsinki
FR|FRA|France|Europe|French;Paris;Macron;Élysée
GA|GAB|Gabon|Sub-Saharan Africa|Gabonese;Libreville
GM|GMB|Gambia|Sub-Saharan Africa|Gambian;Banjul
GE|GEO|Georgia|Europe & Central Asia|Tbilisi;~Georgian;~Georgia
DE|DEU|Germany|Europe|German;Berlin;Bundestag;Frankfurt;Munich
GH|GHA|Ghana|Sub-Saharan Africa|Ghanaian;Accra
GR|GRC|Greece|Europe|Greek;Athens
GD|GRD|Grenada|Latin America & Caribbean|Grenadian
GT|GTM|Guatemala|Latin America & Caribbean|Guatemalan
GN|GIN|Guinea|Sub-Saharan Africa|Conakry;~Guinean;~Guinea
GW|GNB|Guinea-Bissau|Sub-Saharan Africa|Bissau
GY|GUY|Guyana|Latin America & Caribbean|Guyanese;Georgetown
HT|HTI|Haiti|Latin America & Caribbean|Haitian;Port-au-Prince
HN|HND|Honduras|Latin America & Caribbean|Honduran;Tegucigalpa
HU|HUN|Hungary|Europe|Hungarian;Budapest;Orban;Orbán
IS|ISL|Iceland|Europe|Icelandic;Reykjavik
IN|IND|India|South Asia|Indian;New Delhi;Delhi;Mumbai;Modi;Kolkata;Bengaluru;Chennai
ID|IDN|Indonesia|East Asia & Pacific|Indonesian;Jakarta;Bali
IR|IRN|Iran|Middle East & North Africa|Iranian;Tehran;Khamenei
IQ|IRQ|Iraq|Middle East & North Africa|Iraqi;Baghdad;Erbil
IE|IRL|Ireland|Europe|Irish;Dublin
IL|ISR|Israel|Middle East & North Africa|Israeli;Israelis;Jerusalem;Tel Aviv;Netanyahu;IDF
IT|ITA|Italy|Europe|Italian;Rome;Milan
JM|JAM|Jamaica|Latin America & Caribbean|Jamaican;Kingston
JP|JPN|Japan|East Asia & Pacific|Japanese;Tokyo;Osaka
JO|JOR|Jordan|Middle East & North Africa|Jordanian;Amman;~Jordan
KZ|KAZ|Kazakhstan|Europe & Central Asia|Kazakh;Astana;Almaty
KE|KEN|Kenya|Sub-Saharan Africa|Kenyan;Nairobi
KI|KIR|Kiribati|East Asia & Pacific|
KP|PRK|North Korea|East Asia & Pacific|North Korean;Pyongyang;Kim Jong Un;DPRK
KR|KOR|South Korea|East Asia & Pacific|South Korean;Seoul;Republic of Korea
XK|XKX|Kosovo|Europe|Kosovar;Pristina
KW|KWT|Kuwait|Middle East & North Africa|Kuwaiti
KG|KGZ|Kyrgyzstan|Europe & Central Asia|Kyrgyz;Bishkek
LA|LAO|Laos|East Asia & Pacific|Lao;Laotian;Vientiane
LV|LVA|Latvia|Europe|Latvian;Riga
LB|LBN|Lebanon|Middle East & North Africa|Lebanese;Beirut;Hezbollah
LS|LSO|Lesotho|Sub-Saharan Africa|Maseru
LR|LBR|Liberia|Sub-Saharan Africa|Liberian;Monrovia
LY|LBY|Libya|Middle East & North Africa|Libyan;Tripoli;Benghazi
LI|LIE|Liechtenstein|Europe|
LT|LTU|Lithuania|Europe|Lithuanian;Vilnius
LU|LUX|Luxembourg|Europe|Luxembourgish
MG|MDG|Madagascar|Sub-Saharan Africa|Malagasy;Antananarivo
MW|MWI|Malawi|Sub-Saharan Africa|Malawian;Lilongwe
MY|MYS|Malaysia|East Asia & Pacific|Malaysian;Kuala Lumpur
MV|MDV|Maldives|South Asia|Maldivian
ML|MLI|Mali|Sub-Saharan Africa|Malian;Bamako
MT|MLT|Malta|Europe|Maltese;Valletta
MH|MHL|Marshall Islands|East Asia & Pacific|
MR|MRT|Mauritania|Sub-Saharan Africa|Mauritanian;Nouakchott
MU|MUS|Mauritius|Sub-Saharan Africa|Mauritian
MX|MEX|Mexico|Latin America & Caribbean|Mexican;Mexico City;Sheinbaum
FM|FSM|Micronesia|East Asia & Pacific|
MD|MDA|Moldova|Europe|Moldovan;Chisinau
MC|MCO|Monaco|Europe|Monegasque
MN|MNG|Mongolia|East Asia & Pacific|Mongolian;Ulaanbaatar
ME|MNE|Montenegro|Europe|Montenegrin;Podgorica
MA|MAR|Morocco|Middle East & North Africa|Moroccan;Rabat;Casablanca
MZ|MOZ|Mozambique|Sub-Saharan Africa|Mozambican;Maputo
MM|MMR|Myanmar|East Asia & Pacific|Burma;Burmese;Naypyidaw;Yangon
NA|NAM|Namibia|Sub-Saharan Africa|Namibian;Windhoek
NR|NRU|Nauru|East Asia & Pacific|
NP|NPL|Nepal|South Asia|Nepali;Nepalese;Kathmandu
NL|NLD|Netherlands|Europe|Dutch;Amsterdam;The Hague
NZ|NZL|New Zealand|East Asia & Pacific|Wellington;Auckland
NI|NIC|Nicaragua|Latin America & Caribbean|Nicaraguan;Managua
NE|NER|Niger|Sub-Saharan Africa|Nigerien;Niamey
NG|NGA|Nigeria|Sub-Saharan Africa|Nigerian;Abuja;Lagos
MK|MKD|North Macedonia|Europe|Macedonian;Skopje
NO|NOR|Norway|Europe|Norwegian;Oslo
OM|OMN|Oman|Middle East & North Africa|Omani;Muscat
PK|PAK|Pakistan|South Asia|Pakistani;Islamabad;Karachi;Lahore
PW|PLW|Palau|East Asia & Pacific|
PS|PSE|Palestine|Middle East & North Africa|Palestinian;Palestinians;Gaza;West Bank;Ramallah;Hamas
PA|PAN|Panama|Latin America & Caribbean|Panamanian;Panama Canal
PG|PNG|Papua New Guinea|East Asia & Pacific|Port Moresby
PY|PRY|Paraguay|Latin America & Caribbean|Paraguayan;Asuncion;Asunción
PE|PER|Peru|Latin America & Caribbean|Peruvian;Lima
PH|PHL|Philippines|East Asia & Pacific|Philippine;Filipino;Manila
PL|POL|Poland|Europe|Polish;Warsaw
PT|PRT|Portugal|Europe|Portuguese;Lisbon
QA|QAT|Qatar|Middle East & North Africa|Qatari;Doha
RO|ROU|Romania|Europe|Romanian;Bucharest
RU|RUS|Russia|Europe & Central Asia|Russian;Russians;Moscow;Kremlin;Putin;St Petersburg
RW|RWA|Rwanda|Sub-Saharan Africa|Rwandan;Kigali
KN|KNA|Saint Kitts and Nevis|Latin America & Caribbean|
LC|LCA|Saint Lucia|Latin America & Caribbean|
VC|VCT|Saint Vincent and the Grenadines|Latin America & Caribbean|
WS|WSM|Samoa|East Asia & Pacific|Samoan;Apia
SM|SMR|San Marino|Europe|
ST|STP|Sao Tome and Principe|Sub-Saharan Africa|São Tomé
SA|SAU|Saudi Arabia|Middle East & North Africa|Saudi;Saudis;Riyadh;Jeddah
SN|SEN|Senegal|Sub-Saharan Africa|Senegalese;Dakar
RS|SRB|Serbia|Europe|Serbian;Belgrade
SC|SYC|Seychelles|Sub-Saharan Africa|
SL|SLE|Sierra Leone|Sub-Saharan Africa|Freetown
SG|SGP|Singapore|East Asia & Pacific|Singaporean
SK|SVK|Slovakia|Europe|Slovak;Bratislava
SI|SVN|Slovenia|Europe|Slovenian;Ljubljana
SB|SLB|Solomon Islands|East Asia & Pacific|Honiara
SO|SOM|Somalia|Sub-Saharan Africa|Somali;Mogadishu;Al-Shabaab;Somaliland
ZA|ZAF|South Africa|Sub-Saharan Africa|South African;Pretoria;Johannesburg;Cape Town
SS|SSD|South Sudan|Sub-Saharan Africa|South Sudanese;Juba
ES|ESP|Spain|Europe|Spanish;Madrid;Barcelona
LK|LKA|Sri Lanka|South Asia|Sri Lankan;Colombo
SD|SDN|Sudan|Sub-Saharan Africa|Sudanese;Khartoum;Darfur;El Fasher
SR|SUR|Suriname|Latin America & Caribbean|Surinamese;Paramaribo
SE|SWE|Sweden|Europe|Swedish;Stockholm
CH|CHE|Switzerland|Europe|Swiss;Bern;Geneva;Zurich
SY|SYR|Syria|Middle East & North Africa|Syrian;Damascus;Aleppo
TW|TWN|Taiwan|East Asia & Pacific|Taiwanese;Taipei
TJ|TJK|Tajikistan|Europe & Central Asia|Tajik;Dushanbe
TZ|TZA|Tanzania|Sub-Saharan Africa|Tanzanian;Dodoma;Dar es Salaam
TH|THA|Thailand|East Asia & Pacific|Thai;Bangkok
TL|TLS|Timor-Leste|East Asia & Pacific|East Timor;Dili
TG|TGO|Togo|Sub-Saharan Africa|Togolese;Lome;Lomé
TO|TON|Tonga|East Asia & Pacific|Tongan
TT|TTO|Trinidad and Tobago|Latin America & Caribbean|Trinidad
TN|TUN|Tunisia|Middle East & North Africa|Tunisian;Tunis
TR|TUR|Turkiye|Europe & Central Asia|Türkiye;Turkish;Ankara;Istanbul;Erdogan;Erdoğan;~Turkey
TM|TKM|Turkmenistan|Europe & Central Asia|Turkmen;Ashgabat
TV|TUV|Tuvalu|East Asia & Pacific|
UG|UGA|Uganda|Sub-Saharan Africa|Ugandan;Kampala
UA|UKR|Ukraine|Europe|Ukrainian;Ukrainians;Kyiv;Kiev;Zelensky;Zelenskyy;Kharkiv;Odesa
AE|ARE|United Arab Emirates|Middle East & North Africa|UAE;Emirati;Abu Dhabi;Dubai
GB|GBR|United Kingdom|Europe|UK;U.K.;Britain;British;London;England;Scotland;Wales;Downing Street
US|USA|United States|North America|U.S.;US;USA;American;Americans;White House;Pentagon;Congress;New York;California;Texas;Trump;~Washington
UY|URY|Uruguay|Latin America & Caribbean|Uruguayan;Montevideo
UZ|UZB|Uzbekistan|Europe & Central Asia|Uzbek;Tashkent
VU|VUT|Vanuatu|East Asia & Pacific|Port Vila
VA|VAT|Holy See|Europe|Vatican;Pope
VE|VEN|Venezuela|Latin America & Caribbean|Venezuelan;Caracas;Maduro
VN|VNM|Vietnam|East Asia & Pacific|Vietnamese;Hanoi;Ho Chi Minh City;Viet Nam
YE|YEM|Yemen|Middle East & North Africa|Yemeni;Sanaa;Sana'a;Houthi;Houthis;Aden
ZM|ZMB|Zambia|Sub-Saharan Africa|Zambian;Lusaka
ZW|ZWE|Zimbabwe|Sub-Saharan Africa|Zimbabwean;Harare
"""

# Named groupings used by the query engine ("Middle East", "Europe"...).
REGION_GROUPS: dict[str, list[str]] = {
    "middle east": ["IL", "PS", "LB", "SY", "JO", "IQ", "IR", "SA", "YE", "OM", "AE", "QA", "BH", "KW", "EG", "TR"],
    "gulf": ["SA", "AE", "QA", "BH", "KW", "OM"],
    "europe": ["GB", "FR", "DE", "IT", "ES", "PL", "NL", "BE", "SE", "NO", "FI", "DK", "AT", "CH", "IE", "PT", "GR",
               "CZ", "HU", "RO", "BG", "SK", "SI", "HR", "RS", "UA", "BY", "MD", "LT", "LV", "EE"],
    "africa": [],  # filled below from the region column
    "latin america": [],
    "south asia": [],
    "east asia": ["CN", "JP", "KR", "KP", "TW", "MN"],
    "southeast asia": ["ID", "MY", "SG", "TH", "VN", "PH", "MM", "KH", "LA", "BN", "TL"],
    "central asia": ["KZ", "UZ", "KG", "TJ", "TM"],
    "north america": ["US", "CA", "MX"],
    "balkans": ["RS", "BA", "XK", "MK", "AL", "ME", "HR", "SI"],
    "sahel": ["ML", "BF", "NE", "TD", "MR", "SN"],
    "horn of africa": ["ET", "ER", "SO", "DJ", "SD", "SS", "KE"],
}


@dataclass(frozen=True)
class Country:
    iso2: str
    iso3: str
    name: str
    region: str
    aliases: tuple[str, ...] = field(default_factory=tuple)
    ambiguous: tuple[str, ...] = field(default_factory=tuple)


def _load() -> dict[str, Country]:
    out: dict[str, Country] = {}
    for line in _DATA.strip().splitlines():
        iso2, iso3, name, region, al = (line.split("|") + [""])[:5]
        aliases = [a for a in al.split(";") if a]
        clear = tuple(a for a in aliases if not a.startswith("~"))
        amb = tuple(a[1:] for a in aliases if a.startswith("~"))
        out[iso2] = Country(iso2, iso3, name, region, clear, amb)
    return out


COUNTRIES: dict[str, Country] = _load()
ISO3_TO_ISO2 = {c.iso3: c.iso2 for c in COUNTRIES.values()}
REGION_GROUPS["africa"] = [c.iso2 for c in COUNTRIES.values() if c.region == "Sub-Saharan Africa"] + ["EG", "LY", "TN", "DZ", "MA"]
REGION_GROUPS["latin america"] = [c.iso2 for c in COUNTRIES.values() if c.region == "Latin America & Caribbean"]
REGION_GROUPS["south asia"] = [c.iso2 for c in COUNTRIES.values() if c.region == "South Asia"]

# Names that contain another country's name ("South Sudan" contains "Sudan",
# "Niger" vs "Nigeria" is handled by word boundaries) must win first.
_TERMS: list[tuple[str, str, bool]] = []
for _c in COUNTRIES.values():
    _TERMS.append((_c.name, _c.iso2, False))
    _TERMS += [(a, _c.iso2, False) for a in _c.aliases]
    _TERMS += [(a, _c.iso2, True) for a in _c.ambiguous]
_TERMS.sort(key=lambda t: -len(t[0]))
_PATTERN = re.compile(r"(?<![\w])(" + "|".join(re.escape(t[0]) for t in _TERMS) + r")(?![\w])")
_LOOKUP = {t[0]: (t[1], t[2]) for t in _TERMS}


def find_countries(text: str) -> dict[str, float]:
    """Return {iso2: confidence 0..1} for countries mentioned in ``text``.

    The longest alias wins at each position, so "South Sudan" is not read as
    "Sudan". A country named only by an ambiguous alias scores 0.35; a clear
    name scores 0.9 (0.95 if mentioned more than once).
    """
    hits: dict[str, list[bool]] = {}
    for m in _PATTERN.finditer(text or ""):
        iso2, amb = _LOOKUP[m.group(1)]
        hits.setdefault(iso2, []).append(amb)
    out: dict[str, float] = {}
    for iso2, flags in hits.items():
        clear = flags.count(False)
        if clear >= 2:
            out[iso2] = 0.95
        elif clear == 1:
            out[iso2] = 0.9
        else:
            out[iso2] = 0.35
    return out


def resolve_country(query: str) -> str | None:
    """Map a free-text name, ISO2 or ISO3 code to ISO2."""
    q = (query or "").strip()
    if not q:
        return None
    up = q.upper()
    if up in COUNTRIES:
        return up
    if up in ISO3_TO_ISO2:
        return ISO3_TO_ISO2[up]
    low = q.lower()
    for c in COUNTRIES.values():
        if c.name.lower() == low or any(a.lower() == low for a in c.aliases + c.ambiguous):
            return c.iso2
    hits = find_countries(q)
    return max(hits, key=hits.get) if hits else None

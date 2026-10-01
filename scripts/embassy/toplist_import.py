#!/usr/bin/env python3
"""
Top List (Top Toy's community spreadsheet) → Embassy import file (agents/EMBASSY.md "Research strategy" step 1).

    python scripts/embassy/toplist_import.py "C:/Development/toplist/The Top List.xlsx" C:/Development/toplist/embassy-import.json

Reads the .xlsx directly (stdlib only; the CSV export lost its umlauts), and cleans it:
- category → organisation type + track (venue/festival/residency/promoter → live, agency → booking, the rest → promo)
- every cell is searched for emails, URLs, phone numbers and @handles (the columns are mixed up in many rows);
  tracking parameters (fbclid, igsh, utm_*) are dropped; Instagram/forms go to `links`, the first other URL is the website
- multi-person blocks (Powerline Agency: name / email / role lines) become people
- the Vital Weekly writers become people of one organisation "Vital Weekly" instead of 38 organisations
- known duplicates / wrong cities are corrected (Noods → Bristol, LYL → Lyon, HKCR …), the country comes from the city
- "closed" in the notes → tag `closed`

The output is `{ "organisations": [...] }` for POST /api/embassy/import (Embassy app → Import, try a dry run first).
It contains contact data: keep it out of git.
"""
import json
import re
import sys
import xml.etree.ElementTree as ET
import zipfile
from urllib.parse import parse_qsl, urlencode, urlsplit, urlunsplit

NS = {'m': 'http://schemas.openxmlformats.org/spreadsheetml/2006/main'}

CATEGORIES = {
    'venue': ('venue', 'live'), 'residency': ('residency', 'live'), 'promoter': ('promoter', 'live'),
    'festival': ('festival', 'live'), 'agency': ('agency', 'booking'), 'radio': ('radio', 'promo'),
    'podcast': ('podcast', 'promo'), 'newsletter': ('newsletter', 'promo'), 'magazine': ('magazine', 'promo'),
    'blog/magazine': ('magazine', 'promo'), 'blog': ('blog', 'promo'), 'journalist': ('journalist', 'promo'),
    'playlist': ('playlist', 'promo'), 'gallery': ('art_space', 'live'),
}

CITY_COUNTRY = {
    'bern': 'CH', 'wuppertal': 'DE', 'leipzig': 'DE', 'bucharest': 'RO', 'linz': 'AT', 'malmo': 'SE', 'seoul': 'KR',
    'copenhagen': 'DK', 'the hague': 'NL', 'den haag': 'NL', 'de hague': 'NL', 'amsterdam': 'NL', 'paris': 'FR',
    'normandie': 'FR', 'montpellier': 'FR', 'london': 'GB', 'hamburg': 'DE', 'alkmaar': 'NL', 'alkmark': 'NL',
    'zagreb': 'HR', 'rotterdam': 'NL', 'innsbruck': 'AT', 'berlin': 'DE', 'sarajevo': 'BA', 'munich': 'DE',
    'gmunden am traunsee': 'AT', 'tulsa oklahoma': 'US', 'manchester': 'GB', 'marseille': 'FR', 'kyiv': 'UA',
    'duisburg': 'DE', 'los angeles': 'US', 'sydney': 'AU', 'hong kong': 'HK', 'hongkong': 'HK',
    'tallinn and helsinki': 'EE', 'skopje': 'MK', 'brussels': 'BE', 'lyon': 'FR', 'athens': 'GR', 'tbilisi': 'GE',
    'bristol': 'GB', 'vienna': 'AT', 'berlin/brandenburg': 'DE', 'bethlehem': 'PS', 'bremen': 'DE', 'belgrade': 'RS',
    'antwerp': 'BE', 'halle': 'DE', 'graz': 'AT', 'warsaw': 'PL', 'lisbon': 'PT', 'milan': 'IT', 'milano': 'IT',
    'ljubljana': 'SI', 'valparaiso': 'CL', 'istanbul': 'TR', 'odesa': 'UA', 'prague': 'CZ', 'praha': 'CZ',
    'utrecht': 'NL', 'new york': 'US', 'ruhrgebiet': 'DE', 'eupen (be)': 'BE', 'novi sad': 'RS',
    'braga, portugal': 'PT', 'wrocaw, poland': 'PL', 'praca, portugal': 'PT', 'lecce, italy': 'IT', 'wroclaw, poland': 'PL', 'germany': 'DE',
    'wales': 'GB', 'uk': 'GB', 'osterreich': 'AT', 'czech': 'CZ', 'us': 'US',
}

CITY_FIX = {'milano': 'Milan', 'de hague': 'The Hague', 'alkmark': 'Alkmaar', 'praha': 'Prague', 'hongkong': 'Hong Kong',
            'eupen (be)': 'Eupen', 'praca, portugal': 'Braga', 'lecce, italy': 'Lecce', 'amstedam': 'Amsterdam',
            'wroclaw, poland': 'Wrocław', 'wrocaw, poland': 'Wrocław', 'osterreich': '', 'germany': '', 'uk': '', 'czech': '', 'us': '', 'wales': ''}

# name key → (name, city) corrections for entries that are in the list twice
NAME_FIX = {
    'noodsradio': ('Noods Radio', 'Bristol'), 'lylradio': ('LYL Radio', 'Lyon'), 'raheem': ('Radio Raheem', 'Milan'),
    'radioraheem': ('Radio Raheem', 'Milan'), 'hkcrlive': ('Hong Kong Community Radio', 'Hong Kong'),
    'samehads': ('Sameheads', 'Berlin'), 'nts': ('NTS', 'London'), 'vitalyweekly': ('Vital Weekly', ''),
    'weeklyvital': ('Vital Weekly', ''), 'radioradioradio': ('Radioradioradio', 'Amsterdam'),
    'kunstradio': ('Kunstradio (Ö1)', 'Vienna'),
}

# country of entries without a city
NAME_COUNTRY = {'fangoradio': 'IT', 'radiocarbono': 'AR', 'kunstradioo1': 'AT', 'festiwalambientalny': 'PL'}

EMAIL = re.compile(r'(?<![\w.+-])(?:mailto:)?([\w.+-]+@[\w-]+(?:\.[\w-]+)+)', re.I)
URL = re.compile(r'https?://[^\s,]+', re.I)
BARE_URL = re.compile(r'(?<![@\w])((?:www\.)?[a-z0-9-]+(?:\.[a-z0-9-]+)*\.(?:com|net|org|de|at|ch|fr|nl|be|cz|pl|it|uk|live|radio|fm|me|club|cc|ro|dk|si|rs|ru|mk|ge|pt|info)(?:/[^\s,]*)?)', re.I)
HANDLE = re.compile(r'(?<![\w.])@([A-Za-z0-9_.]{2,30})\b')
PHONE = re.compile(r'(?:tel\./fax\.|tel\.?|phone)?\s*(\+?\d[\d ()/-]{7,}\d)', re.I)
DROP_PARAMS = re.compile(r'^(fbclid|igsh|igshid|utm_.*|edit_requested|usp)$', re.I)


def key(text):
    import unicodedata
    t = unicodedata.normalize('NFKD', text.lower()).encode('ascii', 'ignore').decode()
    t = re.sub(r'^(the|le|la|der|die|das)\s+', '', t.strip())
    return re.sub(r'[^a-z0-9]+', '', t)


def city_key(text):
    import unicodedata
    return unicodedata.normalize('NFKD', text.strip().lower()).encode('ascii', 'ignore').decode()


def clean_url(url):
    url = url.rstrip('.,;)')
    parts = urlsplit(url if '://' in url else 'https://' + url)
    query = urlencode([(k, v) for k, v in parse_qsl(parts.query) if not DROP_PARAMS.match(k)])
    return urlunsplit((parts.scheme, parts.netloc.lower(), '' if parts.path == '/' else parts.path, query, ''))


def read_xlsx(path):
    z = zipfile.ZipFile(path)
    shared = [''.join(t.text or '' for t in si.iter('{%s}t' % NS['m'])) for si in ET.fromstring(z.read('xl/sharedStrings.xml'))]
    rows = []
    for row in ET.fromstring(z.read('xl/worksheets/sheet1.xml')).iter('{%s}row' % NS['m']):
        cells = {}
        for c in row.findall('m:c', NS):
            col = re.match(r'[A-Z]+', c.get('r')).group()
            v = c.find('m:v', NS)
            if c.get('t') == 's' and v is not None:
                cells[col] = shared[int(v.text)]
            elif c.get('t') == 'inlineStr':
                cells[col] = ''.join(t.text or '' for t in c.iter('{%s}t' % NS['m']))
            elif v is not None:
                cells[col] = v.text or ''
        rows.append([cells.get(col, '').strip() for col in 'ABCDEFG'])
    return rows


def people_block(text):
    """Multi-line block with several emails: name line before an email, role line after it"""
    lines = [l.strip() for l in text.splitlines() if l.strip()]
    people = []
    for i, line in enumerate(lines):
        m = EMAIL.fullmatch(line)
        if not m:
            continue
        name = lines[i - 1] if i > 0 and not EMAIL.search(lines[i - 1]) and not PHONE.search(lines[i - 1]) else ''
        role = lines[i + 1] if i + 1 < len(lines) and not EMAIL.search(lines[i + 1]) else ''
        if role and i + 2 < len(lines) and EMAIL.fullmatch(lines[i + 2] if i + 2 < len(lines) else ''):
            role = ''  # the next line is the next person's name
        if name:  # an address without a name (mail@…) stays the organisation's email
            people.append({'name': name, 'email': m.group(1).lower(), 'role': role})
    return people


def main(src, dst):
    rows = read_xlsx(src)
    header = next(i for i, r in enumerate(rows) if r[0].lower() == 'category')
    orgs = {}
    vital = {'name': 'Vital Weekly', 'type': 'newsletter', 'city': 'Nijmegen', 'country': 'NL', 'website': 'https://www.vitalweekly.net',
             'links': ['https://www.vitalweekly.net/the-podcast/'], 'tags': ['experimental', 'reviews', 'podcast'], 'source': 'Top List',
             'notes': 'Weekly review newsletter + podcast (Frans de Waard), loosely related to Staalplaat. Acaciastraat 11, 6521 NE Nijmegen.',
             'people': [], 'leads': [{'track': 'promo'}]}
    for r in rows[header + 1:]:
        category, name, city, persons, contacts, info, _ = r
        if not category:
            continue
        cat = category.lower()
        org_type, track = CATEGORIES.get(cat, ('other', 'promo'))
        if cat == 'journalist' and 'vital' in persons.lower():
            vital['people'].append({'name': re.sub(r'\s*\(.*?\)\s*', ' ', name).strip(), 'role': f'writer ({city})' if city else 'writer'})
            continue
        if not name:  # rows with the name in another column
            name = city if cat == 'residency' and city and not city.startswith('http') else ''
            if not name:
                url = URL.search(' '.join(r))
                name = urlsplit(url.group()).netloc.replace('www.', '') if url else ''
            if not name:
                continue
        name = name.strip().lstrip('@') if name.startswith('@') else name.strip()
        nk = key(name)
        if nk in ('vitalyweekly', 'weeklyvital'):  # podcast + newsletter rows of the same thing
            if persons and not URL.search(persons):
                vital['people'].append({'name': persons.strip(), 'role': 'editor'})
            continue
        if nk in NAME_FIX:
            name, fixed_city = NAME_FIX[nk]
            city = fixed_city or city
        ck = city_key(city)
        country = CITY_COUNTRY.get(ck, '') or NAME_COUNTRY.get(key(name), '')
        city = CITY_FIX.get(ck, city.strip().title() if city.islower() else city.strip())
        if city.startswith('http') or '@' in city:
            city = ''

        everything = '\n'.join([persons, contacts, info])
        emails = list(dict.fromkeys(m.group(1).lower() for m in EMAIL.finditer(everything)))
        urls = [clean_url(u) for u in URL.findall(everything)]
        urls += [clean_url(u) for u in BARE_URL.findall(re.sub(r'https?://\S+|\S+@\S+', ' ', everything))]
        handles = [h for h in HANDLE.findall(re.sub(r'\S+@\S+\.\S+', ' ', everything)) if '.' not in h[-4:]]
        urls += [f'https://www.instagram.com/{h}' for h in handles]
        if name.startswith('@') or re.fullmatch(r'@?[a-z0-9_.]+', r[1].strip() or '-'):
            if r[1].strip().startswith('@'):
                urls.append(f'https://www.instagram.com/{r[1].strip().lstrip("@")}')
        urls = list(dict.fromkeys(urls))
        phones = list(dict.fromkeys(p.strip() for p in PHONE.findall(contacts)))
        social = [u for u in urls if re.search(r'instagram\.com|docs\.google\.com|linktr\.ee|litt\.ly|facebook\.com', u)]
        websites = [u for u in urls if u not in social and 'mailto' not in u]

        people = people_block(contacts) if len(emails) > 2 else []
        person_text = URL.sub('', EMAIL.sub('', persons)).replace('mailto:', '').strip(' ,;\n')
        if not people and person_text and len(person_text) < 120 and not BARE_URL.search(person_text):
            for i, pname in enumerate(n.strip() for n in re.split(r',\s*\n|\n|,', person_text) if n.strip()):
                person = {'name': pname}
                if i == 0 and len(emails) >= 2:
                    person['email'] = emails[1]
                people.append(person)
            if len(people) == 1 and len(emails) == 1 and not emails[0].split('@')[0] in ('info', 'contact', 'hello', 'mail', 'booking', 'programming', 'programm', 'editorial', 'office'):
                people[0]['email'] = emails[0]
        org_email = next((e for e in emails if e not in {p.get('email') for p in people}), '')

        notes = [info] if info and not URL.fullmatch(info) and not EMAIL.fullmatch(info) else []
        addr = [l.strip() for l in contacts.splitlines() if l.strip() and not EMAIL.search(l) and not URL.search(l) and not PHONE.search(l)]
        if len(addr) >= 2 and not people_block(contacts):
            notes.append('Address: ' + ', '.join(addr))
        tags = ['closed'] if re.search(r'\bclosed\b', info, re.I) else []
        if cat == 'radio':
            tags.append('web radio')

        org = {
            'name': name, 'type': org_type, 'city': city, 'country': country,
            'website': websites[0] if websites else '', 'email': org_email, 'phone': phones[0] if phones else '',
            'links': social + websites[1:], 'tags': tags, 'source': 'Top List', 'notes': '\n'.join(notes),
            'people': people, 'leads': [{'track': track}],
        }
        k = (key(name), city_key(city))
        if k in orgs:  # merge duplicates of the list here already
            old = orgs[k]
            for f in ('website', 'email', 'phone', 'city', 'country'):
                old[f] = old[f] or org[f]
            old['links'] = list(dict.fromkeys(old['links'] + org['links'] + ([org['website']] if org['website'] and org['website'] != old['website'] else [])))
            old['tags'] = list(dict.fromkeys(old['tags'] + org['tags']))
            old['notes'] = '\n'.join(n for n in dict.fromkeys([old['notes'], org['notes']]) if n)
            old['people'] += [p for p in org['people'] if p not in old['people']]
        else:
            orgs[k] = org
    seen = set()
    vital['people'] = [p for p in vital['people'] if not (key(p['name']) in seen or seen.add(key(p['name'])))]
    out = list(orgs.values()) + ([vital] if vital['people'] else [])
    for o in out:
        o['links'] = [l for l in o['links'] if l != o['website']]
    with open(dst, 'w', encoding='utf-8') as f:
        json.dump({'organisations': out}, f, ensure_ascii=False, indent=1)
    by_type = {}
    for o in out:
        by_type[o['type']] = by_type.get(o['type'], 0) + 1
    reach = sum(1 for o in out if o.get('email') or any(p.get('email') for p in o['people']))
    print(f'{len(out)} organisations, {sum(len(o["people"]) for o in out)} people, {reach} with an email, '
          f'{sum(1 for o in out if o.get("country"))} with a country -> {dst}')
    print(' '.join(f'{t}:{n}' for t, n in sorted(by_type.items(), key=lambda x: -x[1])))


if __name__ == '__main__':
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2])

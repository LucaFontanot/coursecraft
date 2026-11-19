"""Builds the 'Healthy Habits at Work' welcome course: every CourseCraft feature in one friendly module.

Pass 1 runs engine commands (inserts give the ready-made interactions, questions, results and certificate).
Pass 2 edits course.json directly: copy, layout, extra objects, triggers, layers, masters and settings.
Run from the repository root after `cargo build -p coursecraft-cli`:
    python3 examples/healthy-habits/build.py
It rewrites examples/healthy-habits/course.json. Working files go to target/healthy-habits-build/.
The pictures were made with an AI image generator, the narration with ElevenLabs and the
stretch video with ffmpeg from three of the pictures; they live in media/ and are not rebuilt here.
"""
import copy, json, os, re, subprocess, sys

ROOT = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.normpath(os.path.join(ROOT, "../.."))
CLI = os.path.join(REPO, "target/debug/coursecraft-cli")
OUT = os.path.join(REPO, "target/healthy-habits-build")
os.makedirs(OUT, exist_ok=True)

INK, TEAL, DEEP, SAGE, MIST, CORAL, CREAM, WHITE = (
    "#1F2D2B", "#2F6F6A", "#1E4D49", "#A8C3B1", "#E6EFEA", "#B84C30", "#FBF6EE", "#FFFFFF")
BRIGHT = "#E8775A"  # decorative coral, never behind text
BODY, MUTED, CORAL_DK, TEAL_DK = "#33433F", "#5C6B68", "#963A22", "#245A55"
SOFT = {"offset_x": 0, "offset_y": 10, "blur": 28, "color": "#1F2D2B2E"}
LIFT = {"offset_x": 0, "offset_y": 6, "blur": 16, "color": "#B84C3055"}
TEAL_LIFT = {"offset_x": 0, "offset_y": 6, "blur": 14, "color": "#2F6F6A55"}

# ------------------------------------------------------------------ pass 1
SCENES = [("sc_welcome", "Welcome"), ("sc_move", "Move"), ("sc_breathe", "Breathe"), ("sc_connect", "Connect"),
          ("sc_quiz", "Check your knowledge"), ("sc_reflect", "Reflect and finish"), ("sc_extras", "Extras")]
SLIDES = [
    ("sc_welcome", "name", "Nice to meet you"), ("sc_welcome", "habits", "Three small habits"),
    ("sc_move", "desk", "Set up your desk"), ("sc_move", "video", "Three desk stretches"),
    ("sc_move", "eyes", "Rest your eyes"), ("sc_move", "myth", "Myth or fact?"), ("sc_move", "sort", "Helps or hurts?"),
    ("sc_breathe", "breathe", "Breathe with the circle"), ("sc_breathe", "steps", "A one-minute reset"),
    ("sc_breathe", "checkin", "How are you today?"), ("sc_breathe", "resets", "Three quick resets"),
    ("sc_connect", "scenario", "What would you do?"), ("sc_connect", "day", "A healthy day"),
    ("sc_connect", "faq", "Questions people ask"), ("sc_connect", "tips", "Six quick wins"),
    ("sc_connect", "office", "Explore a healthy office"), ("sc_connect", "plan", "Make your plan"),
    ("sc_connect", "more", "Read more"),
]
QUIZ = [("q1", "multiple_choice"), ("q2", "multiple_response"), ("q3", "true_false"), ("q4", "fill_in"),
        ("q5", "sequence"), ("q6", "matching"), ("q7", "hotspot"), ("q8", "drag_drop"), ("q9", "freeform")]
REFLECT = [("r1", "likert"), ("r2", "short_answer"), ("r3", "essay")]
INSERTS = {"myth": "flip_cards", "steps": "process_steps", "resets": "tabs", "scenario": "scenario_choice",
           "day": "timeline", "faq": "accordion", "tips": "reveal_grid", "office": "hotspot_reveal"}

cmds = []
def c(command, **params):
    cmds.append({"command": command, "params": params})

c("course.update", title="Healthy Habits at Work", id="healthy-habits-at-work")
c("course.set_theme", accent=TEAL, background=CREAM, text=INK, font="Inter", bar=DEEP, bar_text=CREAM)
c("theme.set_palette", colors=[
    {"name": "Ink", "color": INK}, {"name": "Teal", "color": TEAL}, {"name": "Deep teal", "color": DEEP},
    {"name": "Sage", "color": SAGE}, {"name": "Mist", "color": MIST}, {"name": "Coral", "color": BRIGHT}, {"name": "Terracotta", "color": CORAL},
    {"name": "Cream", "color": CREAM}])
c("course.set_player", show_title=True, show_menu=True, show_seekbar=True, show_transcript=True,
  show_captions=True, navigation="free")
c("course.set_quiz", passing_percent=80)
for sid, title in SCENES[1:]:
    c("scene.add", id=sid, title=title)
prev = "slide1"
for scene, sid, title in SLIDES:
    if scene == "sc_welcome":
        c("slide.add", id=sid, after=prev, title=title); prev = sid
    else:
        c("slide.add", id=sid, scene=scene, title=title)
for sid, kind in QUIZ:
    c("slide.add", id=sid, scene="sc_quiz", title=f"Question {sid[1:]}")
for sid, kind in REFLECT:
    c("slide.add", id=sid, scene="sc_reflect", title=f"Reflection {sid[1:]}")
c("slide.add", id="finish", scene="sc_reflect", title="Well done")
c("slide.add", id="bonus", scene="sc_extras", title="Bonus: the 20-20-20 rule")
c("variable.add", name="change_minutes", value=60)
for sid, kind in INSERTS.items():
    c("interaction.insert", slide=sid, kind=kind)
for sid, kind in QUIZ + REFLECT:
    c("question.insert", slide=sid, kind=kind)
c("results.insert", after="q9")
c("certificate.insert", after="r3")
json.dump(cmds, open(os.path.join(OUT, "pass1.script.json"), "w"), indent=1)

base = os.path.join(OUT, "base.json")
subprocess.run([CLI, "new", base, "--title", "Healthy Habits at Work"], check=True, capture_output=True)
r = subprocess.run([CLI, "run", base, os.path.join(OUT, "pass1.script.json"), "-o", os.path.join(OUT, "pass1.json")],
                   capture_output=True, text=True)
if r.returncode != 0:
    print(r.stdout[-3000:], r.stderr[-3000:]); sys.exit(1)

# ------------------------------------------------------------------ pass 2 helpers
C = json.load(open(os.path.join(OUT, "pass1.json")))
C["scenes"][0]["id"], C["scenes"][0]["title"] = "sc_welcome", "Welcome"
C["scenes"][0]["slides"][0]["id"] = "welcome"
C["scenes"][0]["slides"][0]["title"] = "Welcome"

def slide(sid):
    for sc in C["scenes"]:
        for s in sc["slides"]:
            if s["id"] == sid:
                return s
    raise KeyError(sid)

def layer(sid, lid):
    for l in slide(sid).get("layers", []):
        if l["id"] == lid:
            return l
    raise KeyError(lid)

def find(sid, oid):
    s = slide(sid)
    for o in s.get("objects", []):
        if o["id"] == oid:
            return o
    for l in s.get("layers", []):
        for o in l.get("objects", []):
            if o["id"] == oid:
                return o
    raise KeyError(f"{sid}/{oid}")

def F(x, y, w, h):
    return {"x": x, "y": y, "w": w, "h": h}

def style(size=20, color=INK, bold=False, **kw):
    s = {"font": "Inter", "size": size, "color": color, "bold": bold}
    s.update(kw)
    return s

def add(sid, obj, lid=None, index=None):
    target = layer(sid, lid) if lid else slide(sid)
    lst = target.setdefault("objects", [])
    if index is None:
        lst.append(obj)
    else:
        lst.insert(index, obj)
    return obj

def obj(oid, name, kind, frame, **extra):
    o = {"id": oid, "name": name, "kind": kind, "frame": frame}
    o.update(extra)
    return o

def text(oid, s, frame, size=20, color=INK, bold=False, name=None, ls=None, paras=None, **st):
    k = {"type": "text", "text": s, "style": style(size, color, bold, **st)}
    if paras:
        k["paragraphs"] = paras
    elif ls:
        k["paragraphs"] = [{"line_spacing": ls} for _ in s.split("\n")]
    return obj(oid, name or oid, k, frame)

def eyebrow(oid, s, x, y, color=CORAL, w=600):
    o = text(oid, s, F(x, y, w, 24), 16, color, True, "Eyebrow", letter_spacing=2)
    o["kind"]["text_style"] = "ts_eyebrow"
    return o

SERIF = "DM Serif Display"   # custom font from Google Fonts (SIL OFL), in media/fonts/

def heading(oid, s, x, y, w=900, h=60, size=40, color=INK):
    o = text(oid, s, F(x, y, w, h), size + 2, color, False, "Title", ls=1.05, font=SERIF)
    o["kind"]["text_style"] = "ts_heading"
    return o

def body(oid, s, x, y, w, h, size=19, color=BODY, name="Body"):
    o = text(oid, s, F(x, y, w, h), size, color, False, name, ls=1.35)
    o["kind"]["text_style"] = "ts_body"
    return o

def shape(oid, frame, fill, kind="rounded_rectangle", name=None, label=None, lstyle=None, adjust=None, outline=None):
    k = {"type": "shape", "shape": kind, "fill": fill}
    if label is not None:
        k["text"] = {"text": label, "style": lstyle or style(18, INK, True, align="center", valign="middle")}
    if adjust:
        k["adjust"] = adjust
    if outline:
        k["outline"] = outline
    return obj(oid, name or oid, k, frame)

def button(oid, label, frame, fill=TEAL, hover=None, name=None):
    o = obj(oid, name or f"{label} button", {"type": "button", "label": label, "fill": fill}, frame)
    o["states"] = [{"name": "hover", "fill": hover or {TEAL: TEAL_DK, CORAL: CORAL_DK, DEEP: INK}.get(fill, fill)}]
    o["shadow"] = LIFT if fill == CORAL else TEAL_LIFT
    return o

def image(oid, src, alt, frame, crop=None, name=None):
    k = {"type": "image", "src": src, "alt": alt}
    if crop:
        k["crop"] = crop
    return obj(oid, name or oid, k, frame)

def anim(o, which, effect, ms=600, start=None, easing="ease_out"):
    o[which] = {"effect": effect, "duration_ms": ms, "easing": easing}
    if start is not None:
        o.setdefault("timing", {})["start_ms"] = start
    return o

FADE = {"type": "fade"}
def fly(d="left"): return {"type": "fly_in", "from": d}
def wipe(d="left"): return {"type": "wipe", "from": d}
def floaty(d="bottom"): return {"type": "float", "from": d}
ZOOM = {"type": "zoom"}

def trig(sid, when, action, cond=None, lid=None):
    t = {"action": action, "when": when}
    if cond:
        t["condition"] = cond
    (layer(sid, lid) if lid else slide(sid)).setdefault("triggers", []).append(t)
    return t

def click(o): return {"event": "click", "object": o}
def jump(target): return {"action": "jump_to_slide", "target": target if target in ("next", "previous") else {"id": target}}
def show(l): return {"action": "show_layer", "layer": l}
def hide(l=None): return {"action": "hide_layer", **({"layer": l} if l else {})}
def state(o, s): return {"action": "change_state", "object": o, "state": s}
def setv(v, value, op="assign"): return {"action": "set_variable", "variable": v, "op": op, "value": value}
def cmp(v, op, value): return {"compare": {"variable": v, "op": op, "value": value}}
def ostate(o, s, is_=True): return {"object_state": {"object": o, "state": s, "is": is_}}

def var(name, value):
    C.setdefault("variables", [])
    C["variables"] = [v for v in C["variables"] if v["name"] != name] + [{"name": name, "value": value}]

def new_layer(sid, lid, name, **opts):
    l = {"id": lid, "name": name, "duration_ms": opts.pop("duration_ms", 5000), "objects": [],
         "hide_other_layers": opts.pop("hide_other_layers", True),
         "pause_base_timeline": opts.pop("pause_base_timeline", False),
         "prevent_base_clicks": opts.pop("prevent_base_clicks", False)}
    l.update(opts)
    slide(sid).setdefault("layers", []).append(l)
    return l

def set_label(sid, oid, s, **st):
    o = find(sid, oid)
    k = o["kind"]
    if k["type"] == "text":
        k["text"] = s; k.pop("runs", None); k["style"].update(st)
    elif k["type"] == "shape":
        k.setdefault("text", {"text": "", "style": style(18)})
        k["text"]["text"] = s; k["text"].pop("runs", None); k["text"]["style"].update(st)
    elif k["type"] == "button":
        k["label"] = s
    return o

def setup(sid, bg=CREAM, transition=None, notes=None, master=True, duration=None):
    s = slide(sid)
    s["background"] = bg
    if transition:
        s["transition"] = transition
    if notes:
        s["notes"] = notes
    if master:
        s["master"] = "m_content"
    if duration:
        s["duration_ms"] = duration
    return s

T_FADE = {"type": "fade", "duration_ms": 500}
def T_PUSH(d="right"): return {"type": "push", "from": d, "duration_ms": 600}
def T_COVER(d="bottom"): return {"type": "cover", "from": d, "duration_ms": 600}

def next_button(sid, x=1040, y=600, label="Continue", oid=None, lid=None, fill=TEAL, w=160):
    oid = oid or f"{sid}_next"
    add(sid, button(oid, label, F(x, y, w, 52), fill), lid)
    trig(sid, click(oid), jump("next"), lid=lid)
    return oid

# ------------------------------------------------------------------ course-wide settings
C["text_styles"] = [
    {"id": "ts_heading", "name": "Heading", "style": style(42, INK, False, font=SERIF), "paragraphs": {"line_spacing": 1.05}},
    {"id": "ts_eyebrow", "name": "Eyebrow", "style": style(16, CORAL, True, letter_spacing=2)},
    {"id": "ts_body", "name": "Body", "style": style(19, BODY), "paragraphs": {"line_spacing": 1.35}},
]
C["glossary"] = [
    {"id": "g_ergonomics", "term": "Ergonomics", "definition": "Fitting the workspace to the person: chair, desk, screen and tools set up so your body stays comfortable."},
    {"id": "g_microbreak", "term": "Microbreak", "definition": "A pause of one to five minutes to stand, stretch, look away or breathe."},
    {"id": "g_202020", "term": "20-20-20 rule", "definition": "Every 20 minutes, look at something 20 feet (about 6 metres) away for 20 seconds."},
    {"id": "g_box", "term": "Paced breathing", "definition": "Breathing in, holding and breathing out to a steady count, such as 4-4-6, to calm the body."},
]
C["resources"] = [
    {"id": "res_who", "title": "WHO: Physical activity fact sheet", "target": {"kind": "url", "url": "https://www.who.int/news-room/fact-sheets/detail/physical-activity"}},
    {"id": "res_coursecraft", "title": "CourseCraft on GitHub", "target": {"kind": "url", "url": "https://github.com/ad1-kamath/coursecraft"}},
]
var("learner_name", "friend")
for v, val in [("habits_seen", 0.0), ("seen_move", False), ("seen_breathe", False), ("seen_connect", False),
               ("tips_opened", 0.0), ("tip_1", False), ("tip_2", False), ("tip_3", False),
               ("sorted", 0.0), ("stress", 4.0), ("sleep_hours", 7.0), ("first_habit", "move"),
               ("reminders", True), ("change_minutes", 60.0), ("breaths", 0.0)]:
    var(v, val)

# Master: a quiet footer on every content slide.
C["masters"] = [{
    "id": "m_content", "title": "Content", "background": CREAM, "duration_ms": 5000,
    "objects": [
        obj("m_strip", "Accent strip", {"type": "shape", "shape": "rectangle", "fill": SAGE}, F(0, 0, 8, 720), decorative=True),
        obj("m_tab", "Accent tab", {"type": "shape", "shape": "rectangle", "fill": BRIGHT}, F(0, 40, 8, 64), decorative=True),
    ]}]
for o in C["masters"][0]["objects"]:
    o["decorative"] = True

# ------------------------------------------------------------------ 1. Welcome
s = "welcome"
setup(s, CREAM, T_FADE, "Welcome to Healthy Habits at Work. In the next few minutes, you'll pick up small habits that make every workday calmer.", master=False, duration=8000)
add(s, anim(image("w_hero", "hero.jpg", "A person stretching happily beside a sunny desk", F(600, 0, 680, 720),
                  crop={"x": 0.2, "y": 0.0, "w": 0.527, "h": 1.0}, name="Hero picture"), "entrance", FADE, 900))
add(s, obj("w_curve", "Decorative curve", {"type": "path", "points": [{"x": 0, "y": 1}, {"x": 0.35, "y": 0.2}, {"x": 0.7, "y": 0.8}, {"x": 1, "y": 0}],
           "style": "curved", "outline": {"color": SAGE, "width": 3, "dash": "dot"}, "end_end": "dot"}, F(80, 104, 220, 40), decorative=True))
add(s, anim(eyebrow("w_eyebrow", "WELLBEING SERIES  ·  A COURSECRAFT SAMPLE", 80, 176), "entrance", fly("left"), 500, 200))
add(s, anim(text("w_title", "Healthy Habits\nat Work", F(76, 212, 520, 150), 64, INK, False, "Title", ls=0.82, font=SERIF), "entrance", fly("left"), 700, 350))
add(s, anim(body("w_sub", "Small changes to how you sit, breathe and take breaks add up to calmer, more focused days.", 80, 370, 440, 90, 20, "#4A5B58", "Subtitle"), "entrance", FADE, 600, 700))
st = add(s, anim(button("w_start", "Start the course  →", F(80, 484, 236, 58), CORAL), "entrance", ZOOM, 500, 1000))
add(s, anim(shape("w_chip", F(336, 492, 176, 42), MIST, name="Duration chip", label="15 min  ·  5 lessons",
                  lstyle=style(16, DEEP, True, align="center", valign="middle"), adjust=[0.5]), "entrance", FADE, 500, 1100))
add(s, anim(text("w_hint", "Press Enter to start", F(82, 560, 260, 24), 16, MUTED, False, "Keyboard hint"), "entrance", FADE, 500, 1400))
add(s, obj("w_voice", "Welcome narration", {"type": "audio", "src": "welcome.mp3", "autoplay": True, "captions": "welcome.vtt", "volume": 0.9}, F(80, 610, 36, 36)))
add(s, button("w_listen", "Listen again", F(124, 610, 140, 36), DEEP, name="Listen again button"))
add(s, button("w_pause", "Pause", F(272, 610, 90, 36), DEEP, name="Pause narration button"))
trig(s, click("w_start"), jump("next"))
trig(s, {"event": "key_pressed", "key": "enter"}, jump("next"))
trig(s, click("w_listen"), {"action": "play_media", "object": "w_voice"})
trig(s, click("w_pause"), {"action": "pause_media", "object": "w_voice"})

# ------------------------------------------------------------------ 2. Name
s = "name"
setup(s, CREAM, T_PUSH("right"), "Type your name so we can greet you and put it on your certificate.")
add(s, image("n_pic", "office.jpg", "A bright, plant-filled open-plan office", F(680, 64, 520, 560), crop={"x": 0.25, "y": 0.0, "w": 0.5, "h": 1.0}, name="Office picture"))
slide(s)["objects"][-1]["shadow"] = SOFT
add(s, anim(eyebrow("n_eyebrow", "BEFORE WE START", 80, 150), "entrance", FADE, 400))
add(s, anim(heading("n_title", "What should we\ncall you?", 78, 180, 560, 120, 44), "entrance", fly("bottom"), 600, 150))
add(s, body("n_body", "We'll use your name to greet you and on your certificate at the end.", 80, 312, 500, 60))
add(s, obj("n_entry", "Name entry", {"type": "text_entry", "variable": "learner_name", "placeholder": "Your first name", "max_chars": 40}, F(80, 392, 420, 56), description="Your first name"))
add(s, text("n_greet", "Nice to meet you, %learner_name%!", F(80, 466, 540, 40), 24, TEAL, True, "Greeting"), )
find(s, "n_greet")["initial_state"] = "hidden"
anim(find(s, "n_greet"), "entrance", ZOOM, 400)
next_button(s, 80, 528, "Continue  →", w=180)
trig(s, {"event": "variable_changes", "variable": "learner_name"}, state("n_greet", "normal"), cmp("learner_name", "ne", ""))
trig(s, {"event": "variable_changes", "variable": "learner_name"}, state("n_greet", "hidden"), cmp("learner_name", "eq", ""))

# ------------------------------------------------------------------ 3. Habits (cards, layers, groups, meter)
s = "habits"
setup(s, CREAM, T_FADE, "Open each card to see what the habit is about. The Next button appears when you've opened all three.")
add(s, eyebrow("h_eyebrow", "WHAT YOU'LL PRACTISE", 80, 52))
add(s, heading("h_title", "Three small habits, one better day", 78, 80))
add(s, body("h_hint", "Tap each card to open it.", 80, 140, 500, 30, 18, MUTED, "Hint"))
add(s, obj("h_meter", "Habits explored", {"type": "progress_meter", "variable": "habits_seen", "min": 0, "max": 3, "style": "ring", "show_value": True}, F(1132, 60, 68, 68), description="Habits explored"))
add(s, text("h_meter_label", "explored", F(1100, 132, 132, 22), 16, MUTED, True, "Meter label", align="center"))
cards = [("move", "Move", "Change position every 30 minutes and set your desk up to fit you.", "desk.jpg", "A tidy ergonomic desk",
          "Sitting still is the problem, not sitting. Stand for calls, walk to a colleague instead of messaging, and set your chair so your feet rest flat."),
         ("breathe", "Breathe", "Use a one-minute breathing pattern to reset between tasks.", "breathe.jpg", "A person taking a mindful breath",
          "Slow breathing tells your body it is safe. Four counts in, four held, six out, three times, takes about a minute."),
         ("connect", "Connect", "Swap one meeting a week for a walk and talk outside.", "team.jpg", "Colleagues on a walking meeting",
          "A walking one-to-one gets you both moving, outside and talking more freely. Most catch-ups don't need slides.")]
for i, (key, head, blurb, src, alt, detail) in enumerate(cards):
    x = 80 + i * 384
    g = f"card_{key}"
    card = add(s, shape(f"h_card_{key}", F(x, 186, 352, 440), WHITE, name=f"{head} card", adjust=[0.06]))
    card["shadow"] = SOFT
    card["states"] = [{"name": "hover", "fill": "#F3F8F5"}]
    card["description"] = f"Open the {head} habit"
    add(s, image(f"h_pic_{key}", src, alt, F(x + 16, 202, 320, 196), crop={"x": 0.1, "y": 0.05, "w": 0.8, "h": 0.895}, name=f"{head} picture"))
    num = add(s, shape(f"h_num_{key}", F(x + 24, 376, 44, 44), CORAL if i == 1 else TEAL, "ellipse", name=f"{head} number",
                       label=str(i + 1), lstyle=style(18, WHITE, True, align="center", valign="middle")))
    num["states"] = [{"name": "done", "fill": DEEP, "text": "✓"}]
    add(s, text(f"h_head_{key}", head, F(x + 24, 434, 300, 40), 26, INK, True, f"{head} heading"))
    add(s, body(f"h_body_{key}", blurb, x + 24, 478, 304, 110, 17, "#4A5B58", f"{head} text"))
    for oid in (f"h_card_{key}", f"h_pic_{key}", f"h_num_{key}", f"h_head_{key}", f"h_body_{key}"):
        find(s, oid)["group"] = g
        anim(find(s, oid), "entrance", floaty("bottom"), 600, 150 + i * 150)
    lid = f"h_layer_{key}"
    new_layer(s, lid, f"{head} details", prevent_base_clicks=True)
    add(s, shape(f"h_dim_{key}", F(0, 0, 1280, 720), "#1F2D2B99", "rectangle", name="Dim"), lid)
    find(s, f"h_dim_{key}")["decorative"] = True
    panel = add(s, anim(shape(f"h_panel_{key}", F(190, 110, 900, 480), WHITE, name="Panel", adjust=[0.04]), "entrance", ZOOM, 350), lid)
    panel["shadow"] = SOFT
    add(s, image(f"h_big_{key}", src, alt, F(214, 134, 400, 432), crop={"x": 0.22, "y": 0.0, "w": 0.56, "h": 1.0}, name="Picture"), lid)
    add(s, eyebrow(f"h_l_eyebrow_{key}", f"HABIT {i + 1} OF 3", 650, 170), lid)
    add(s, heading(f"h_l_title_{key}", head, 648, 198, 400, 60, 40), lid)
    add(s, body(f"h_l_body_{key}", detail, 650, 266, 400, 200), lid)
    add(s, button(f"h_close_{key}", "Got it", F(650, 494, 140, 50)), lid)
    trig(s, click(f"h_close_{key}"), hide(), lid=lid)
    trig(s, {"event": "key_pressed", "key": "escape"}, hide(), lid=lid)
    flag = f"seen_{key}"
    for part in (f"h_card_{key}", f"h_pic_{key}"):
        trig(s, click(part), setv("habits_seen", 1.0, "add"), cmp(flag, "eq", False))
        trig(s, click(part), setv(flag, True))
        trig(s, click(part), show(lid))
        trig(s, click(part), state(f"h_num_{key}", "done"))
add(s, button("habits_next", "Next lesson  →", F(1040, 140, 160, 40)))
find(s, "habits_next")["initial_state"] = "hidden"
anim(find(s, "habits_next"), "entrance", ZOOM, 400)
trig(s, click("habits_next"), jump("next"))
trig(s, {"event": "variable_changes", "variable": "habits_seen"}, state("habits_next", "normal"), cmp("habits_seen", "ge", 3))
find(s, "h_meter")["frame"] = F(1132, 40, 68, 68)
find(s, "h_meter_label")["frame"] = F(1100, 110, 132, 22)
find(s, "habits_next")["frame"] = F(1000, 138, 200, 40)
for k in ("move", "breathe", "connect"):
    trig(s, {"event": "timeline_starts"}, setv(f"seen_{k}", False))
trig(s, {"event": "timeline_starts"}, setv("habits_seen", 0.0))

# ------------------------------------------------------------------ 4. Desk markers
s = "desk"
setup(s, MIST, T_PUSH("right"), "Tap each plus sign on the picture to see why it matters.", master=False)
add(s, image("d_desk", "desk.jpg", "Side view of an ergonomic desk setup", F(0, 0, 760, 720), crop={"x": 0.38, "y": 0.0, "w": 0.589, "h": 1.0}, name="Desk picture"))
for i, (mx, my, title, bd) in enumerate([
        (60, 196, "Screen at eye level", "The top of the monitor sits at or just below your eyes, about an arm's length away."),
        (270, 318, "Elbows at 90°", "Keyboard close, shoulders relaxed, wrists straight."),
        (150, 640, "Feet flat", "Both feet rest on the floor or a footrest.")], 1):
    add(s, anim(obj(f"d_m{i}", f"Marker {i}", {"type": "marker", "title": title, "body": bd, "icon": "plus", "color": BRIGHT}, F(mx, my, 40, 40)), "entrance", ZOOM, 400, 300 + i * 200))
    trig(s, {"event": "marker_opened", "object": f"d_m{i}"}, setv("tips_opened", 1.0, "add"), cmp(f"tip_{i}", "eq", False))
    trig(s, {"event": "marker_opened", "object": f"d_m{i}"}, setv(f"tip_{i}", True))
add(s, eyebrow("d_eyebrow", "LESSON 1  ·  MOVE", 820, 110, TEAL))
add(s, heading("d_title", "Make the desk fit you, not the other way round", 818, 140, 400, 130, 34))
add(s, text("d_list", "Tap each + on the picture.\nStand up or stretch every 30 minutes.\nKeep water within reach.", F(820, 300, 390, 160), 19, BODY, False, "Checklist",
            paras=[{"space_after": 14, "line_spacing": 1.3}, {"list": "bullet", "space_after": 10, "line_spacing": 1.3}, {"list": "bullet", "space_after": 10, "line_spacing": 1.3}]))
add(s, shape("d_count", F(820, 476, 220, 40), WHITE, name="Tips counter", label="%tips_opened% of 3 tips opened", lstyle=style(16, DEEP, True, align="center", valign="middle"), adjust=[0.5]))
next_button(s, 820, 560, "Next lesson  →", w=190)
for i in (1, 2, 3):
    trig(s, {"event": "timeline_starts"}, setv(f"tip_{i}", False))
trig(s, {"event": "timeline_starts"}, setv("tips_opened", 0.0))

# ------------------------------------------------------------------ 5. Video with cues and captions
s = "video"
setup(s, CREAM, T_COVER("bottom"), "Follow along with three gentle stretches you can do at your desk.", duration=15000)
add(s, eyebrow("v_eyebrow", "LESSON 1  ·  MOVE", 80, 52))
add(s, heading("v_title", "Three stretches you can do at your desk", 78, 80, 1000))
vid = add(s, obj("v_video", "Stretch video", {"type": "video", "src": "stretch.mp4", "autoplay": False, "captions": "stretch.vtt", "cues": [
    {"id": "c_neck", "at_ms": 300, "name": "Neck stretch"}, {"id": "c_shoulder", "at_ms": 4700, "name": "Shoulder stretch"},
    {"id": "c_twist", "at_ms": 9100, "name": "Seated twist"}]}, F(80, 160, 720, 402)))
vid["shadow"] = SOFT
add(s, button("v_play", "Play", F(80, 584, 110, 46)))
add(s, button("v_pause", "Pause", F(200, 584, 110, 46), DEEP))
add(s, text("v_cc", "Captions: use the CC button in the player bar.", F(330, 596, 470, 24), 16, MUTED, False, "Captions hint"))
trig(s, click("v_play"), {"action": "play_media", "object": "v_video"})
trig(s, click("v_pause"), {"action": "pause_media", "object": "v_video"})
steps = [("neck", "Neck stretch", "Tilt your head towards one shoulder, then the other."),
         ("shoulder", "Shoulder stretch", "Pull one arm across your chest. Switch sides."),
         ("twist", "Seated twist", "Sit tall, hold the backrest and turn gently.")]
for i, (key, title, bd) in enumerate(steps):
    y = 160 + i * 136
    row = add(s, shape(f"v_row_{key}", F(840, y, 360, 120), WHITE, name=f"{title} row", adjust=[0.08]))
    row["states"] = [{"name": "active", "fill": "#FCE6DE"}, {"name": "done", "fill": MIST}]
    row["shadow"] = SOFT
    n = add(s, shape(f"v_num_{key}", F(860, y + 20, 36, 36), MUTED, "ellipse", name=f"{title} number", label=str(i + 1), lstyle=style(16, WHITE, True, align="center", valign="middle")))
    n["states"] = [{"name": "active", "fill": CORAL}, {"name": "done", "fill": TEAL, "text": "✓"}]
    add(s, text(f"v_head_{key}", title, F(910, y + 18, 270, 30), 20, INK, True, f"{title} heading"))
    add(s, text(f"v_text_{key}", bd, F(910, y + 50, 270, 60), 16, BODY, False, f"{title} text", ls=1.3))
    trig(s, {"event": "cue_reached", "object": "v_video", "cue": f"c_{key}"}, state(f"v_row_{key}", "active"))
    trig(s, {"event": "cue_reached", "object": "v_video", "cue": f"c_{key}"}, state(f"v_num_{key}", "active"))
    if i > 0:
        prev = steps[i - 1][0]
        trig(s, {"event": "cue_reached", "object": "v_video", "cue": f"c_{key}"}, state(f"v_row_{prev}", "done"))
        trig(s, {"event": "cue_reached", "object": "v_video", "cue": f"c_{key}"}, state(f"v_num_{prev}", "done"))
trig(s, {"event": "media_ends", "object": "v_video"}, state("v_row_twist", "done"))
trig(s, {"event": "media_ends", "object": "v_video"}, state("v_num_twist", "done"))
new_layer(s, "v_done", "All done", duration_ms=15000)
add(s, anim(shape("v_toast", F(840, 572, 360, 70), DEEP, name="Done toast", label="Nice work! Your back says thanks.", lstyle=style(18, CREAM, True, align="center", valign="middle"), adjust=[0.2]), "entrance", fly("bottom"), 400), "v_done")
trig(s, {"event": "media_ends", "object": "v_video"}, show("v_done"))
next_button(s, 1040, 600, oid="video_next")
find(s, "video_next")["frame"] = F(1040, 600, 160, 52)
slide(s)["objects"] = [o for o in slide(s)["objects"] if o["id"] != "video_next"]
add(s, button("video_next", "Continue", F(840, 584, 160, 46)), )
slide(s)["objects"] = [o for o in slide(s)["objects"] if not (o["id"] == "video_next" and o["frame"]["y"] == 584)]
add(s, button("video_next", "Continue", F(1040, 584, 160, 46)))
# keep only one copy of the continue button
seen = set(); objs = []
for o in slide(s)["objects"]:
    if o["id"] in seen:
        continue
    seen.add(o["id"]); objs.append(o)
slide(s)["objects"] = objs
find(s, "v_toast")["frame"] = F(840, 572, 180, 70)
find(s, "v_toast")["kind"]["text"]["text"] = "Nice work!"

# ------------------------------------------------------------------ 6. Eyes: timer + lightbox
s = "eyes"
setup(s, CREAM, T_FADE, "Start the 20-second timer and look at something far away until it ends.", duration=30000)
add(s, image("e_pic", "eyes.jpg", "A person resting their eyes by looking out of a window", F(0, 0, 600, 720), crop={"x": 0.2, "y": 0.0, "w": 0.47, "h": 1.0}, name="Eyes picture"))
add(s, eyebrow("e_eyebrow", "LESSON 1  ·  MOVE", 660, 96))
add(s, heading("e_title", "Give your eyes a 20-second holiday", 658, 124, 560, 110, 38))
add(s, body("e_body", "Screens keep your eyes focused close up. Every 20 minutes, look at something about 6 metres away for 20 seconds.", 660, 240, 540, 90))
ring = add(s, shape("e_ring", F(660, 352, 170, 170), WHITE, "ellipse", name="Timer ring", outline={"color": SAGE, "width": 6}))
ring["glow"] = {"size": 10, "color": "#A8C3B166"}
add(s, obj("e_timer", "Eye timer", {"type": "timer", "duration_ms": 20000, "format": "seconds", "autostart": False,
           "style": style(56, DEEP, True, align="center", valign="middle")}, F(660, 392, 170, 90), description="Seconds left"))
add(s, text("e_timer_label", "seconds", F(660, 470, 170, 24), 16, MUTED, True, "Timer label", align="center"))
add(s, button("e_start", "Start", F(860, 372, 120, 46), CORAL))
add(s, button("e_pause", "Pause", F(990, 372, 110, 46), DEEP))
add(s, button("e_reset", "Reset", F(860, 428, 120, 46), DEEP))
add(s, button("e_why", "Why 20-20-20?", F(860, 484, 240, 40), TEAL))
trig(s, click("e_start"), {"action": "start_timer", "timer": "e_timer"})
trig(s, click("e_pause"), {"action": "pause_timer", "timer": "e_timer"})
trig(s, click("e_reset"), {"action": "reset_timer", "timer": "e_timer"})
trig(s, click("e_reset"), hide("e_done"))
trig(s, click("e_why"), {"action": "lightbox_slide", "slide": "bonus"})
new_layer(s, "e_done", "Time's up", duration_ms=30000)
add(s, anim(shape("e_toast", F(660, 552, 440, 64), TEAL, name="Done toast", label="Welcome back! Your eyes thank you.", lstyle=style(18, WHITE, True, align="center", valign="middle"), adjust=[0.25]), "entrance", fly("bottom"), 400), "e_done")
trig(s, {"event": "timer_ends", "timer": "e_timer"}, show("e_done"))
next_button(s, 1120, 600, "Next", w=100)

# ------------------------------------------------------------------ 7. Myth or fact (flip cards insert)
s = "myth"
setup(s, CREAM, T_PUSH("right"), "Tap a card to flip it and find out.")
add(s, eyebrow("m_eyebrow", "QUICK CHECK", 80, 52))
add(s, heading("m_title", "Myth or fact? Tap a card to find out", 78, 80, 1000))
flips = [("“Sitting up straight all day is best.”", "Myth. Any posture held for hours gets tiring. Your best posture is your next one."),
         ("“Short breaks make you more productive.”", "Fact. A few minutes away from the screen helps you refocus and make fewer mistakes."),
         ("“You need a gym to stay active at work.”", "Myth. Stairs, walking calls and a stretch at your desk all count.")]
for i, (front, back) in enumerate(flips, 1):
    o = find(s, f"flip_card{i}")
    o["frame"] = F(80 + (i - 1) * 384, 190, 352, 340)
    edge, tint = [(TEAL, MIST), (CORAL, "#FCE6DE"), (DEEP, "#E3EEF0")][i - 1]
    o["kind"]["fill"] = WHITE
    o["kind"]["outline"] = {"color": edge, "width": 3, "dash": "solid"}
    o["kind"]["text"]["text"] = front
    o["kind"]["text"]["style"].update(style(24, INK, True, align="center", valign="middle"))
    o["shadow"] = SOFT
    for stt in o.get("states", []):
        if stt["name"] != "hover":
            stt["text"] = back
            stt["fill"] = tint
    anim(o, "entrance", floaty("bottom"), 600, 200 + i * 150)
add(s, text("m_hint", "Tap a card again to flip it back.", F(80, 560, 1120, 30), 16, MUTED, False, "Hint", align="center"))
next_button(s)

# ------------------------------------------------------------------ 8. Sort: draggable objects, intersections, dropped_on
s = "sort"
setup(s, CREAM, T_FADE, "Drag each card onto Helps or Hurts.")
add(s, eyebrow("so_eyebrow", "TRY IT", 80, 52))
add(s, heading("so_title", "Helps or hurts? Drag each card to a side", 78, 80, 1000))
bins = [("so_helps", "Helps", 640, MIST, TEAL), ("so_hurts", "Hurts", 930, "#FCE6DE", CORAL)]
for bid, label, x, fill, edge in bins:
    b = add(s, shape(bid, F(x, 170, 270, 430), fill, name=f"{label} area", adjust=[0.06], outline={"color": edge, "width": 2, "dash": "dash"},
                     label=label, lstyle=style(26, edge, True, align="center", valign="top")))
    b["states"] = [{"name": "active", "fill": WHITE}]
items = [("so_water", "Refill your water bottle", "so_helps"), ("so_lunch", "Eat lunch at your keyboard", "so_hurts"),
         ("so_stairs", "Take the stairs", "so_helps"), ("so_scroll", "Scroll in bed at midnight", "so_hurts")]
for i, (iid, label, right) in enumerate(items):
    it = add(s, shape(iid, F(80 + (i % 2) * 240, 200 + (i // 2) * 110, 220, 84), WHITE, name=label, adjust=[0.2],
                      label=label, lstyle=style(18, INK, True, align="center", valign="middle")))
    it["shadow"] = SOFT
    it["states"] = [{"name": "right", "fill": "#DDEDE3", "text": label + "  ✓"}, {"name": "wrong", "fill": "#FCE6DE", "text": "Try the other side"}]
    it["drag"] = {"targets": ["so_helps", "so_hurts"], "snap": False}
    wrong = "so_hurts" if right == "so_helps" else "so_helps"
    trig(s, {"event": "dropped_on", "object": iid, "target": right}, setv("sorted", 1.0, "add"), ostate(iid, "right", False))
    trig(s, {"event": "dropped_on", "object": iid, "target": right}, state(iid, "right"))
    trig(s, {"event": "dropped_on", "object": iid, "target": wrong}, state(iid, "wrong"))
    for bid, *_ in bins:
        trig(s, {"event": "objects_intersect", "a": iid, "b": bid}, state(bid, "active"))
        trig(s, {"event": "objects_stop_intersecting", "a": iid, "b": bid}, state(bid, "normal"))
add(s, shape("so_count", F(80, 440, 220, 40), MIST, name="Sorted counter", label="%sorted% of 4 sorted", lstyle=style(16, DEEP, True, align="center", valign="middle"), adjust=[0.5]))
new_layer(s, "so_done", "All sorted", duration_ms=5000)
add(s, anim(shape("so_toast", F(80, 500, 460, 80), TEAL, name="Done toast", label="All sorted! Small swaps like these add up.", lstyle=style(18, WHITE, True, align="center", valign="middle"), adjust=[0.25]), "entrance", ZOOM, 400), "so_done")
trig(s, {"event": "variable_changes", "variable": "sorted"}, show("so_done"), cmp("sorted", "ge", 4))
trig(s, {"event": "timeline_starts"}, setv("sorted", 0.0))
next_button(s, 1040, 620, w=160)
find(s, "sort_next")["frame"] = F(80, 610, 160, 46)

# ------------------------------------------------------------------ 9. Breathe: Lottie, timeline_reaches, motion paths, narration
s = "breathe"
setup(s, DEEP, T_FADE, "Let's try it together. Breathe in for four. Hold for four. And slowly out, for six.", master=False, duration=20000)
slide(s)["revisit"] = "reset"
add(s, eyebrow("b_eyebrow", "LESSON 2  ·  BREATHE", 80, 96, SAGE))
add(s, text("b_title", "Breathe with\nthe circle", F(78, 126, 480, 140), 52, CREAM, True, "Title", ls=1.0))
add(s, text("b_body", "Watch the circle grow as you breathe in, rest as you hold and shrink as you breathe out.", F(80, 286, 420, 90), 19, "#D5E3DC", False, "Body", ls=1.35))
lab = add(s, text("b_label", "Get comfortable…", F(80, 420, 460, 56), 34, WHITE, True, "Breathing cue"))
lab["states"] = [{"name": "in", "text": "Breathe in · 4"}, {"name": "hold", "text": "Hold · 4"}, {"name": "out", "text": "Breathe out · 6"}, {"name": "again", "text": "Lovely. Once more?"}]
add(s, obj("b_circle", "Breathing circle", {"type": "lottie", "src": "breathe-circle.json", "loop": False}, F(640, 80, 560, 560), description="A circle that grows and shrinks with your breath"))
leaf = add(s, obj("b_leaf", "Floating leaf", {"type": "shape", "shape": "ellipse", "fill": BRIGHT}, F(912, 612, 16, 16), decorative=True))
leaf["motion_paths"] = [
    {"id": "p_up", "points": [{"x": 0, "y": 0}, {"x": 0, "y": -560}], "duration_ms": 4000, "easing": "ease_in_out", "relative": True},
    {"id": "p_down", "points": [{"x": 0, "y": 0}, {"x": 0, "y": 560}], "duration_ms": 6000, "easing": "ease_in_out", "relative": True}]
add(s, obj("b_voice", "Breathing narration", {"type": "audio", "src": "breathe.mp3", "autoplay": True, "captions": "breathe.vtt"}, F(80, 520, 36, 36)))
add(s, button("b_mute", "Pause voice", F(124, 520, 140, 36), TEAL))
trig(s, click("b_mute"), {"action": "stop_media", "object": "b_voice"})
add(s, button("b_again", "Breathe again", F(80, 590, 180, 50), CORAL))
find(s, "b_again")["initial_state"] = "hidden"
add(s, text("b_count", "Breaths taken: %breaths%", F(280, 602, 300, 28), 18, SAGE, True, "Breath counter"))
trig(s, {"event": "timeline_reaches", "ms": 5000}, state("b_label", "in"))
trig(s, {"event": "timeline_reaches", "ms": 5000}, {"action": "move_on_path", "object": "b_leaf", "path": "p_up"})
trig(s, {"event": "timeline_reaches", "ms": 9000}, state("b_label", "hold"))
trig(s, {"event": "timeline_reaches", "ms": 13000}, state("b_label", "out"))
trig(s, {"event": "timeline_reaches", "ms": 13000}, {"action": "move_on_path", "object": "b_leaf", "path": "p_down"})
trig(s, {"event": "animation_completes", "object": "b_leaf", "path": "p_down"}, setv("breaths", 1.0, "add"))
trig(s, {"event": "timeline_ends"}, state("b_label", "again"))
trig(s, {"event": "timeline_ends"}, state("b_again", "normal"))
trig(s, click("b_again"), jump("breathe"))
add(s, button("breathe_next", "Continue", F(1040, 600, 160, 46), CORAL))
trig(s, click("breathe_next"), jump("next"))

# ------------------------------------------------------------------ 10. Steps (process steps insert)
s = "steps"
setup(s, CREAM, T_PUSH("right"), "Use Next and Back to walk through the four steps.")
steps4 = [("Sit tall", "Feet flat, hands resting in your lap. Let your shoulders drop away from your ears."),
          ("Breathe in · 4", "Breathe in slowly through your nose while you count to four."),
          ("Hold · 4", "Hold the breath gently, without strain, for another count of four."),
          ("Breathe out · 6", "Let the air out through your mouth for a count of six. Repeat three times.")]
for i, (title, detail) in enumerate(steps4, 1):
    set_label(s, f"step_label{i}", title)
    set_label(s, f"step_panel{i}_title1", title, size=30)
    set_label(s, f"step_panel{i}_detail1", detail, size=22, color=BODY)
    find(s, f"step_panel{i}_detail1")["frame"] = F(110, 310, 620, 200)
    add(s, image(f"st_pic{i}", "breathe.jpg", "A person breathing calmly", F(800, 250, 380, 330), crop={"x": 0.3, "y": 0.0, "w": 0.4, "h": 1.0}, name=f"Step {i} picture"), f"step_layer{i}")
    find(s, f"st_pic{i}")["shadow"] = SOFT
for i in range(1, 5):
    pnl = find(s, f"step_panel{i}1")
    pnl["kind"]["fill"] = WHITE
    pnl["kind"]["outline"] = {"color": SAGE, "width": 2, "dash": "solid"}
    pnl["shadow"] = SOFT
find(s, "step_back1")["frame"] = F(80, 604, 120, 46)
find(s, "step_next1")["frame"] = F(212, 604, 120, 46)
next_button(s, 1040, 600, oid="steps_next")
find(s, "steps_next")["frame"] = F(1040, 610, 160, 46)

# ------------------------------------------------------------------ 11. Check-in: slider, dial, meter, conditions
s = "checkin"
setup(s, CREAM, T_FADE, "Move the slider and the dial. The page reacts to your answers.")
add(s, eyebrow("ci_eyebrow", "CHECK IN", 80, 52))
add(s, heading("ci_title", "How are you feeling today?", 78, 80))
p1 = add(s, shape("ci_card1", F(80, 160, 540, 440), WHITE, name="Stress card", adjust=[0.05])); p1["shadow"] = SOFT
add(s, text("ci_q1", "How stressed do you feel right now?", F(112, 188, 480, 30), 21, INK, True, "Stress question"))
add(s, obj("ci_slider", "Stress slider", {"type": "slider", "variable": "stress", "min": 0, "max": 10, "step": 1}, F(112, 250, 476, 40), description="Stress from 0 to 10"))
add(s, text("ci_ends", "Calm                                                                        Very stressed", F(112, 296, 476, 22), 16, MUTED, False, "Slider ends"))
add(s, obj("ci_meter", "Stress meter", {"type": "progress_meter", "variable": "stress", "min": 0, "max": 10, "style": "bar"}, F(112, 340, 476, 14), description="Stress level"))
add(s, text("ci_value", "Stress level: %stress% out of 10", F(112, 368, 476, 30), 20, DEEP, True, "Stress value"))
tip = add(s, shape("ci_tip", F(112, 420, 476, 150), MIST, name="Stress tip", adjust=[0.08],
                   label="You're doing fine. Keep taking short breaks to stay that way.", lstyle=style(18, DEEP, False, align="left", valign="middle")))
tip["states"] = [{"name": "high", "fill": "#FCE6DE", "text": "That's a lot. Try the one-minute breathing reset before your next task."}]
trig(s, {"event": "variable_changes", "variable": "stress"}, state("ci_tip", "high"), cmp("stress", "ge", 7))
trig(s, {"event": "variable_changes", "variable": "stress"}, state("ci_tip", "normal"), cmp("stress", "lt", 7))
p2 = add(s, shape("ci_card2", F(660, 160, 540, 440), WHITE, name="Sleep card", adjust=[0.05])); p2["shadow"] = SOFT
add(s, text("ci_q2", "How many hours did you sleep?", F(692, 188, 480, 30), 21, INK, True, "Sleep question"))
add(s, obj("ci_dial", "Sleep dial", {"type": "dial", "variable": "sleep_hours", "min": 4, "max": 10, "step": 0.5}, F(830, 236, 200, 200), description="Hours of sleep"))
add(s, text("ci_sleep", "%sleep_hours% hours", F(692, 448, 476, 34), 24, DEEP, True, "Sleep value", align="center"))
note = add(s, text("ci_sleep_note", "That's a healthy amount of rest.", F(692, 492, 476, 70), 18, BODY, False, "Sleep note", align="center"))
note["states"] = [{"name": "low", "text": "Under 7 hours? Try winding down 30 minutes earlier tonight."}]
trig(s, {"event": "variable_changes", "variable": "sleep_hours"}, state("ci_sleep_note", "low"), cmp("sleep_hours", "lt", 7))
trig(s, {"event": "variable_changes", "variable": "sleep_hours"}, state("ci_sleep_note", "normal"), cmp("sleep_hours", "ge", 7))
next_button(s, 1040, 612, w=160)
find(s, "checkin_next")["frame"] = F(1040, 612, 160, 46)

# ------------------------------------------------------------------ 12. Resets (tabs insert)
s = "resets"
setup(s, CREAM, T_PUSH("right"), "Use the tabs to explore three quick resets.")
add(s, eyebrow("rs_eyebrow", "LESSON 2  ·  BREATHE", 80, 40))
for i, (label, head, detail, src, alt) in enumerate([
        ("Breathe", "One minute of paced breathing", "Breathe in for four, hold for four and out for six. Three rounds take about a minute and calm your body down.", "breathe.jpg", "A person breathing calmly"),
        ("Rest your eyes", "The 20-20-20 rule", "Every 20 minutes, look at something about 6 metres away for 20 seconds. Your eye muscles relax when they focus far away.", "eyes.jpg", "A person looking out of a window"),
        ("Drink water", "Keep a bottle in reach", "Mild dehydration makes it harder to concentrate. Refilling your bottle is also a good excuse to stand up.", "water.jpg", "A person refilling a water bottle")], 1):
    b = find(s, f"tab_button{i}")
    b["kind"]["label"] = label
    b["kind"]["fill"] = MUTED
    b["frame"] = F(80 + (i - 1) * 376, 76, 360, 52)
    b["states"] = [{"name": "selected", "fill": DEEP}, {"name": "hover", "fill": "#4A5856"}]
    ct = find(s, f"tab_content{i}")
    ct["kind"]["text"] = detail
    ct["kind"]["style"].update(style(20, BODY))
    ct["kind"]["paragraphs"] = [{"line_spacing": 1.4}]
    ct["frame"] = F(80, 240, 500, 200)
    lid = f"tab_layer{i}"
    add(s, heading(f"tab_head{i}", head, 80, 166, 520, 60, 32), lid, 0)
    add(s, image(f"tab_pic{i}", src, alt, F(640, 160, 560, 440), crop={"x": 0.1, "y": 0.0, "w": 0.8, "h": 1.0}, name=f"{label} picture"), lid)
    find(s, f"tab_pic{i}")["shadow"] = SOFT
    anim(find(s, f"tab_pic{i}"), "entrance", wipe("left"), 500)
next_button(s, 80, 600, w=160)
find(s, "resets_next")["frame"] = F(80, 600, 160, 46)

# ------------------------------------------------------------------ 13. Scenario
s = "scenario"
setup(s, CREAM, T_FADE, "Choose what you'd do. You can try the other answers too.")
set_label(s, "scenario_title1", "What would you do?", color=WHITE)
set_label(s, "scenario_body1", "Your 3 pm meeting is a one-to-one with a colleague. You've both been on screens since 9.", color=MIST)
find(s, "scenario_panel1")["kind"]["fill"] = DEEP
find(s, "scenario_panel1")["kind"].pop("outline", None)
for i, label in enumerate(["Keep the video call: it's quicker", "Suggest a walking meeting outside", "Move it to tomorrow morning"], 1):
    set_label(s, f"scenario_choice{i}", label)
set_label(s, "scenario_feedback_text1", "A call works, but it keeps you both at the screen. There's a way to get the same talk and some movement.")
set_label(s, "scenario_feedback_text2", "Good thinking. A one-to-one rarely needs slides, and a walk gets you both moving, outside and talking more freely.")
set_label(s, "scenario_feedback_text3", "Delaying doesn't help either of you today. Look for a way to keep the meeting and add a break.")
for i, t in enumerate(["Not the best choice", "Great choice!", "Not the best choice"], 1):
    set_label(s, f"scenario_feedback_title{i}", t)
add(s, image("sc_pic", "stairs.jpg", "Two colleagues taking the stairs together", F(0, 0, 1280, 720), name="Background picture"), index=0)
find(s, "sc_pic")["opacity"] = 0.18
find(s, "sc_pic")["decorative"] = True
next_button(s, 1040, 610, w=160)

# ------------------------------------------------------------------ 14. Day (timeline insert)
s = "day"
setup(s, CREAM, T_PUSH("right"), "Tap each time to see a habit that fits that part of the day.")
add(s, eyebrow("dy_eyebrow", "LESSON 3  ·  CONNECT", 80, 40))
day = [("9:00", "Set up", "Adjust your chair and screen before you open email. Two minutes now saves an aching neck later.", "desk.jpg"),
       ("11:00", "Refill", "Stand up, refill your water and look out of a window on the way.", "water.jpg"),
       ("13:00", "Walk", "Take lunch away from your desk. A ten-minute walk after eating helps the afternoon dip.", "team.jpg"),
       ("15:00", "Stretch", "Three desk stretches: neck, shoulders and a seated twist.", "stretch.jpg"),
       ("18:00", "Switch off", "Close the laptop, write tomorrow's first task down and let the day end.", "evening.jpg")]
for i, (t, title, detail, src) in enumerate(day, 1):
    set_label(s, f"timeline_year{i}", t, color=CORAL, bold=True, size=18)
    find(s, f"timeline_point{i}")["description"] = f"{t}: {title}"
    set_label(s, f"timeline_title{i}", title, color=INK, bold=True, size=18)
    set_label(s, f"timeline_panel{i}_title1", title, size=30)
    set_label(s, f"timeline_panel{i}_detail1", detail, size=21, color=BODY)
    find(s, f"timeline_panel{i}_detail1")["frame"] = F(112, 290, 520, 200)
    find(s, f"timeline_panel{i}_title1")["frame"] = F(112, 236, 520, 44)
    add(s, image(f"dy_pic{i}", src, title, F(680, 224, 500, 400), crop={"x": 0.15, "y": 0.0, "w": 0.7, "h": 1.0}, name=f"{title} picture"), f"timeline_layer{i}")
    anim(find(s, f"dy_pic{i}"), "entrance", FADE, 400)
add(s, heading("dy_title", "A healthy day, hour by hour", 78, 66))
for o in slide(s)["objects"]:
    if o["id"].startswith("timeline_"):
        o["frame"]["y"] += 90
for i in range(1, 6):
    L = layer(s, f"timeline_layer{i}")
    for o in L["objects"]:
        o["frame"]["y"] += 60
    find(s, f"timeline_panel{i}1")["frame"]["h"] = 340
    find(s, f"timeline_panel{i}1")["kind"]["fill"] = WHITE
    find(s, f"timeline_panel{i}1")["shadow"] = SOFT
    find(s, f"timeline_panel{i}_title1")["frame"] = F(112, 274, 520, 44)
    find(s, f"timeline_panel{i}_detail1")["frame"] = F(112, 326, 520, 200)
    find(s, f"dy_pic{i}")["frame"] = F(700, 268, 470, 300)
next_button(s, 1040, 610, w=160)

# ------------------------------------------------------------------ 15. FAQ (accordion insert)
s = "faq"
setup(s, CREAM, T_FADE, "Open a question to read the answer.")
qa = [("I'm too busy for breaks. Where do I start?", "Start with one: stand up for every phone call. It costs no time and gets you moving several times a day."),
      ("Do standing desks fix everything?", "No. Standing all day is tiring too. The win is changing position often, whatever desk you have."),
      ("How do I remember to do all this?", "Tie each habit to something you already do: refill water after each meeting, breathe before opening email.")]
for i, (q, a) in enumerate(qa, 1):
    for oid in [f"accordion_header{i}"] + [f"accordion_h{j}_{i}1" for j in (1, 2, 3)]:
        try:
            o = find(s, oid)
        except KeyError:
            continue
        o["kind"]["label"] = q
        o["kind"]["fill"] = TEAL if oid.startswith(f"accordion_h{i}_") else DEEP
        o["states"] = [st for st in o.get("states", []) if st["name"] != "hover"] + [{"name": "hover", "fill": TEAL_DK}]
    ct = find(s, f"accordion_content{i}")
    ct["kind"]["text"] = a
    ct["kind"]["style"].update(style(20, BODY))
for sl in ("accordion_header", ):
    pass
for o in slide(s)["objects"]:
    o["frame"]["y"] += 90
for i in (1, 2, 3):
    for o in layer(s, f"accordion_layer{i}1")["objects"]:
        o["frame"]["y"] += 90
add(s, eyebrow("fq_eyebrow", "QUESTIONS PEOPLE ASK", 80, 52))
add(s, heading("fq_title", "Straight answers to common worries", 78, 80))
for i in (1, 2, 3):
    add(s, eyebrow(f"fq_eyebrow_l{i}", "QUESTIONS PEOPLE ASK", 80, 52), f"accordion_layer{i}1")
    add(s, heading(f"fq_title_l{i}", "Straight answers to common worries", 78, 80), f"accordion_layer{i}1")
next_button(s, 1040, 610, w=160)
for i in (1, 2, 3):
    next_button(s, 1040, 610, oid=f"faq_next_l{i}", lid=f"accordion_layer{i}1", w=160)

# ------------------------------------------------------------------ 16. Tips (reveal grid insert)
s = "tips"
setup(s, CREAM, T_PUSH("right"), "Tap each tile to reveal a quick win.")
add(s, eyebrow("tp_eyebrow", "SIX QUICK WINS", 80, 40))
tips = [("Stand for calls", "Every phone call is a chance to stand and walk."),
        ("Stairs first", "One flight up beats the lift for your heart."),
        ("Bottle on desk", "If you can see it, you'll drink it."),
        ("Walking 1:1s", "Talk side by side instead of face to screen."),
        ("Lunch away", "Eat anywhere but your keyboard."),
        ("Last task first", "Write tomorrow's first task down, then log off.")]
for i, (front, back) in enumerate(tips, 1):
    o = find(s, f"grid_tile_shape{i}")
    o["kind"]["fill"] = WHITE
    o["kind"]["outline"] = {"color": SAGE, "width": 2, "dash": "solid"}
    o["kind"]["text"]["text"] = front
    o["kind"]["text"]["style"].update(style(24, DEEP, True, align="center", valign="middle"))
    o["shadow"] = SOFT
    o["frame"]["y"] = o["frame"]["y"] + 10
    for stt in o.get("states", []):
        if stt["name"] == "Revealed":
            stt["fill"] = "#DDEDE3"
            stt["text"] = back
set_label(s, "grid_counter1", "%grid1_explored% of 6 revealed", color=DEEP, bold=True)
next_button(s, 1040, 610, w=160)
find(s, "tips_next")["frame"] = F(1080, 30, 120, 44)

# ------------------------------------------------------------------ 17. Office (hotspot reveal insert, picture fill)
s = "office"
setup(s, CREAM, T_FADE, "Tap the numbers on the picture to explore the office.")
pic = find(s, "hotspot_picture1")
pic["kind"]["fill_style"] = {"type": "picture", "src": "office.jpg", "mode": "stretch"}
pic["kind"]["text"]["text"] = ""
pic["kind"].pop("outline", None)
pic["frame"] = F(80, 90, 1120, 560)
pic["description"] = "An open-plan office with plants, big windows and a sofa"
add(s, eyebrow("of_eyebrow", "EXPLORE", 80, 40))
add(s, text("of_title", "Explore a healthy office", F(220, 32, 600, 40), 28, INK, True, "Title"))
spots = [(170, 300, "Green corners", "Plants soften a room and give your eyes something natural to rest on."),
         (680, 230, "Daylight", "Sit near windows when you can: daylight helps your energy and your sleep."),
         (1060, 470, "A place to pause", "A sofa away from desks invites real breaks and quick chats.")]
for i, (x, y, title, detail) in enumerate(spots, 1):
    m = find(s, f"hotspot_marker{i}")
    m["frame"] = F(x, y, 56, 56)
    m["kind"]["fill"] = CORAL
    m["kind"]["text"]["style"]["color"] = WHITE
    m["shadow"] = LIFT
    set_label(s, f"hotspot_callout{i}_title1", title)
    set_label(s, f"hotspot_callout{i}_detail1", detail)
    co = find(s, f"hotspot_callout{i}1")
    co["kind"]["fill"] = WHITE
    co["kind"]["outline"] = {"color": SAGE, "width": 2, "dash": "solid"}
    co["shadow"] = SOFT
next_button(s, 1080, 30, "Next", w=120)
find(s, "office_next")["frame"] = F(1080, 30, 120, 44)

# ------------------------------------------------------------------ 18. Plan: table, radio group, checkbox
s = "plan"
setup(s, CREAM, T_PUSH("right"), "Choose your first habit and look at a sample week.")
add(s, eyebrow("pl_eyebrow", "MAKE IT STICK", 80, 52))
add(s, heading("pl_title", "Your plan for next week", 78, 80))
def cell(t, fill=None, bold=False, color=INK):
    d = {"text": t, "style": style(16, color, bold, align="center", valign="middle"), "valign": "middle"}
    if fill:
        d["fill"] = fill
    return d
rows = [["", "Move", "Breathe", "Connect"], ["Mon", "Stand for calls", "Before email", "—"], ["Tue", "Stairs", "After lunch", "Walking 1:1"],
        ["Wed", "Stand for calls", "Before email", "—"], ["Thu", "Stretch at 3", "After lunch", "Coffee outside"], ["Fri", "Walk at lunch", "End of day", "Thank someone"]]
table = {"type": "table", "columns": [0.16, 0.28, 0.28, 0.28], "header_row": True,
         "rows": [{"height": 1, "cells": [cell(t, TEAL if r == 0 and j > 0 else (MIST if j == 0 else None), r == 0 or j == 0, WHITE if r == 0 and j > 0 else INK) for j, t in enumerate(row)]} for r, row in enumerate(rows)],
         "borders": {"outer": {"color": SAGE, "width": 2}, "inner": {"color": "#D7E5DD", "width": 1}}}
add(s, obj("pl_table", "Sample week", table, F(80, 160, 640, 400), description="A sample week of habits"))
p = add(s, shape("pl_card", F(760, 160, 440, 400), WHITE, name="Choice card", adjust=[0.05])); p["shadow"] = SOFT
add(s, text("pl_q", "Which habit will you start with?", F(792, 186, 380, 30), 21, INK, True, "Habit question"))
add(s, obj("pl_radio", "First habit", {"type": "radio_group", "variable": "first_habit", "options": [
    {"label": "Move more", "value": "move"}, {"label": "Breathe and reset", "value": "breathe"}, {"label": "Connect with people", "value": "connect"}]},
    F(792, 230, 380, 130), description="Your first habit"))
add(s, text("pl_pick", "Great pick: %first_habit%.", F(792, 372, 380, 30), 19, TEAL, True, "Your pick"))
add(s, obj("pl_check", "Reminders", {"type": "checkbox", "variable": "reminders", "label": "Remind me to take a break every hour"}, F(792, 430, 380, 40)))
note = add(s, text("pl_note", "We'll nudge you hourly.", F(792, 480, 380, 50), 16, MUTED, False, "Reminder note"))
note["states"] = [{"name": "off", "text": "No reminders. You've got this."}]
trig(s, {"event": "variable_changes", "variable": "reminders"}, state("pl_note", "off"), cmp("reminders", "eq", False))
trig(s, {"event": "variable_changes", "variable": "reminders"}, state("pl_note", "normal"), cmp("reminders", "eq", True))
next_button(s, 1040, 600, w=160)

# ------------------------------------------------------------------ 19. Read more: scroll panel, web object, open_url, text link
s = "more"
setup(s, CREAM, T_FADE, "Scroll the article, or open the links to read more.")
add(s, eyebrow("mo_eyebrow", "READ MORE", 80, 52))
add(s, heading("mo_title", "Why small habits work", 78, 80))
add(s, obj("mo_panel", "Article", {"type": "scroll_panel", "content_height": 980}, F(80, 160, 600, 440), description="Article: why small habits work"))
y = 176
for i, (h, ptxt) in enumerate([
        ("Start tiny", "Big changes fade when motivation dips. A habit so small it feels silly, like one stretch after each meeting, survives busy weeks."),
        ("Stack it", "Attach the new habit to one you already have. Refill water after every call. Breathe before you open your inbox."),
        ("Make it visible", "Put the water bottle on your desk and your walking shoes by the door. What you see, you do."),
        ("Celebrate", "Notice when you did it. A small 'nice' to yourself teaches your brain the habit is worth keeping."),
        ("Expect off days", "Missing once doesn't matter. Missing twice is the start of a new habit, so just pick it up again tomorrow.")]):
    o1 = add(s, text(f"mo_h{i}", h, F(104, y, 540, 34), 24, DEEP, True, f"Article heading {i + 1}"))
    o2 = add(s, body(f"mo_p{i}", ptxt, 104, y + 40, 540, 110, 18, BODY, f"Article text {i + 1}"))
    o1["panel"] = o2["panel"] = "mo_panel"
    y += 168
add(s, obj("mo_web", "WHO fact sheet", {"type": "web", "source": {"type": "url", "url": "https://www.who.int/news-room/fact-sheets/detail/physical-activity"}}, F(720, 160, 480, 300)))
add(s, button("mo_open", "Open the WHO fact sheet", F(720, 480, 300, 48)))
trig(s, click("mo_open"), {"action": "open_url", "url": "https://www.who.int/news-room/fact-sheets/detail/physical-activity"})
lk = add(s, text("mo_link", "Prefer a refresher? Go back to the breathing circle.", F(720, 548, 480, 30), 16, BODY, False, "Link text"))
lk["kind"]["runs"] = [{"text": "Prefer a refresher? Go back to the "}, {"text": "breathing circle", "link": {"type": "slide", "slide": "breathe"}}, {"text": "."}]
next_button(s, 1040, 600, "Start the quiz  →", w=200)
find(s, "more_next")["frame"] = F(1000, 600, 200, 48)

# ------------------------------------------------------------------ Quiz
QSPEC = {
    "q1": dict(prompt="You've been at your desk for two hours. What's best now?", choices=[
        ("a", "Stand, stretch and look at something far away", True), ("b", "Push through: breaks break focus", False),
        ("c", "Check your phone for a few minutes", False), ("d", "Wait until lunch, then rest", False)], attempts=2, shuffle=True),
    "q2": dict(prompt="Which of these count as a movement break? Choose all that apply.", choices=[
        ("a", "Taking the stairs", True), ("b", "Standing for a phone call", True), ("c", "Scrolling social media", False), ("d", "A walking meeting", True)], scoring="partial"),
    "q3": dict(prompt="True or false: the best posture is the one you hold all day.", choices=[("true", "True", False), ("false", "False", True)]),
    "q4": dict(prompt="Look 20 feet away for how many seconds? Type a number.", accepted=["20", "twenty"]),
    "q5": dict(prompt="Put the paced breathing steps in order.", choices=[
        ("a", "Sit tall and relax your shoulders", False), ("b", "Breathe in for four", False), ("c", "Hold for four", False), ("d", "Breathe out for six", False)]),
    "q6": dict(prompt="Match each habit to the best time of day.", pairs=[("p1", "Set up your desk", "Start of the day"), ("p2", "Walk outside", "Lunch break"), ("p3", "Write tomorrow's first task", "End of the day")]),
    "q7": dict(prompt="Click where the top of the screen should line up with your eyes."),
    "q8": dict(prompt="Drag each item to the habit it belongs to."),
    "q9": dict(prompt="Set the slider to how often you should change position.", ),
    "r1": dict(prompt="This course gave me ideas I can use tomorrow."),
    "r2": dict(prompt="In a few words: which habit will you try first?"),
    "r3": dict(prompt="Describe one change to your workday, and when."),
}
QSIDE = {"q1": "desk.jpg", "q2": "stairs.jpg", "q3": "stretch.jpg", "q4": "eyes.jpg", "q5": "breathe.jpg", "q6": "evening.jpg",
         "q8": "team.jpg", "q9": "office.jpg", "r1": "hero.jpg", "r2": "water.jpg", "r3": "evening.jpg"}
NQ = len(QUIZ)
for n, (sid, kind) in enumerate(QUIZ + REFLECT, 1):
    sp = QSPEC[sid]
    setup(sid, CREAM, T_FADE if n % 2 else T_PUSH("right"))
    q = slide(sid)["question"]
    q["prompt"] = sp["prompt"]
    if "choices" in sp:
        q["choices"] = [{"id": i, "text": t, "correct": cr} for i, t, cr in sp["choices"]]
    if "accepted" in sp:
        q["accepted"] = sp["accepted"]
    if "pairs" in sp:
        q["pairs"] = [{"id": i, "left": l, "right": r} for i, l, r in sp["pairs"]]
    for k in ("attempts", "shuffle", "scoring"):
        if k in sp:
            q[k] = sp[k]
    label = f"QUESTION {n} OF {NQ}" if sid.startswith("q") else f"REFLECTION {sid[1:]} OF 3  ·  NOT SCORED"
    add(sid, eyebrow(f"{sid}_eyebrow", label, 80, 32), index=0)
    if sid in QSIDE:
        side = add(sid, image(f"{sid}_side", QSIDE[sid], "", F(1040, 0, 240, 720), crop={"x": 0.42, "y": 0.0, "w": 0.24, "h": 1.0}, name="Side picture"), index=0)
        side["decorative"] = True
    sub = find(sid, "button2" if sid == "q8" else "button1")
    sub["kind"]["label"] = "Submit" if sid.startswith("q") else "Save answer"
    sub["kind"]["fill"] = CORAL
    sub["frame"] = F(80, 600, 170, 52)
    sub["states"] = [{"name": "hover", "fill": CORAL_DK}]
    sub["shadow"] = LIFT
    # feedback layers become friendly toasts with a Continue button
    for lid, toid, fill, msg in [("Correct1", "feedback_text1", TEAL, None), ("Incorrect1", "feedback_text2", "#B5523A", None)]:
        if lid not in [l["id"] for l in slide(sid).get("layers", [])]:
            continue
        L = layer(sid, lid)
        L["prevent_base_clicks"] = True
        add(sid, anim(shape(f"{sid}_{lid}_toast", F(280, 590, 720, 72), fill, name="Feedback panel", adjust=[0.2]), "entrance", fly("bottom"), 350), lid, 0)
        t = find(sid, toid)
        t["frame"] = F(306, 604, 520, 44)
        t["kind"]["style"].update(style(20, WHITE, True, valign="middle"))
        anim(t, "entrance", fly("bottom"), 350)
        add(sid, button(f"{sid}_{lid}_go", "Continue  →", F(840, 604, 140, 44), WHITE if False else DEEP), lid)
        trig(sid, click(f"{sid}_{lid}_go"), jump("next"), lid=lid)
    fb = {"q1": ("Correct! A short movement break rests your eyes and your back.", "Not quite. Your body needs a change of position more often than that."),
          "q2": ("Yes! Anything that gets you up and moving counts.", "Look again: scrolling keeps you sitting still."),
          "q3": ("Right. Your best posture is your next one.", "It's false: any posture held for hours gets tiring."),
          "q4": ("Correct: 20 seconds is enough.", "It's 20 seconds: 20-20-20."),
          "q5": ("Perfect order. Breathe easy!", "Not quite: sit, in, hold, out."),
          "q6": ("All matched!", "Some pairs are off. Desk first, walk at lunch, plan at the end."),
          "q7": ("Spot on: the top of the screen at eye level.", "Look higher: the top of the monitor."),
          "q8": ("Nicely sorted!", "A couple are in the wrong place."),
          "q9": ("Yes: every 30 minutes or so.", "Aim for about every 30 minutes."),
          "r1": ("Thanks for your answer!", "Thanks for your answer!"),
          "r2": ("Thanks! Writing it down makes it more likely.", "Thanks!"),
          "r3": ("Thanks! A plan with a time attached is much easier to keep.", "Thanks!")}
    ok, no = fb[sid]
    try:
        set_label(sid, "feedback_text1", ok)
        set_label(sid, "feedback_text2", no)
    except KeyError:
        pass

# hotspot on the desk picture
s = "q7"
hs = find(s, "image1")
hs["kind"].update({"src": "desk.jpg", "alt": "Side view of an ergonomic desk setup", "crop": {"x": 0.25, "y": 0.0, "w": 0.75, "h": 1.0}})
hs["frame"] = F(80, 150, 640, 420)
slide(s)["question"]["regions"] = [{"id": "r_screen", "shape": "ellipse", "x": 130, "y": 160, "w": 200, "h": 90, "correct": True}]
add(s, text("q7_hint", "Tip: the screen is on the desk on the right of the picture.", F(760, 160, 260, 90), 16, MUTED, False, "Hint", ls=1.3))

# drag and drop question
s = "q8"
slide(s)["objects"] = [o for o in slide(s)["objects"] if o["id"] not in ("button1", "shape1", "text1")]
dd_items = [("dd_water", "Water bottle", "dd_move"), ("dd_stairs", "Stairs", "dd_move"), ("dd_count", "Count of four", "dd_breathe"), ("dd_walk", "Walking 1:1", "dd_connect")]
for j, (tid, label) in enumerate([("dd_move", "Move"), ("dd_breathe", "Breathe"), ("dd_connect", "Connect")]):
    add(s, shape(tid, F(80 + j * 310, 340, 290, 200), [MIST, "#FCE6DE", "#E3EEF0"][j], name=f"{label} target", adjust=[0.08],
                 outline={"color": [TEAL, CORAL, DEEP][j], "width": 2, "dash": "dash"}, label=label, lstyle=style(22, [TEAL, CORAL_DK, DEEP][j], True, align="center", valign="top")))
for j, (iid, label, tgt) in enumerate(dd_items):
    b = add(s, button(iid, label, F(80 + j * 230, 170, 210, 56), DEEP, name=label))
q = slide(s)["question"]
find(s, "button2")["frame"] = F(80, 600, 170, 52)
# button1 was the default drag item; give it back its submit role and rebuild the lists
q["drag_items"] = [{"item": iid, "target": tgt} for iid, _, tgt in dd_items]
q["drop_targets"] = ["dd_move", "dd_breathe", "dd_connect"]
q["snap_to_center"] = False if False else True
q["one_per_target"] = False

# freeform question graded by a slider
s = "q9"
add(s, obj("q9_slider", "Minutes slider", {"type": "slider", "variable": "change_minutes", "min": 0, "max": 120, "step": 15}, F(80, 220, 860, 44), description="Minutes between position changes"))
add(s, text("q9_value", "Every %change_minutes% minutes", F(80, 280, 860, 40), 28, DEEP, True, "Slider value"))
add(s, text("q9_ends", "0 min                                                                                                                                       120 min", F(80, 330, 860, 22), 16, MUTED, False, "Slider ends"))
slide(s)["question"]["correct_when"] = {"all": [cmp("change_minutes", "ge", 15), cmp("change_minutes", "le", 30)]}
trig(s, {"event": "timeline_starts"}, setv("change_minutes", 60.0))

# likert scale labels
slide("r1")["question"]["choices"] = [{"id": str(i), "text": t, "correct": False} for i, t in enumerate(["Strongly disagree", "Disagree", "Not sure", "Agree", "Strongly agree"], 1)]

# results slide restyle
s = "results1"
setup(s, CREAM, T_FADE, "Your quiz score. Retry or review your answers.", master=True)
slide(s)["title"] = "Your results"
set_label(s, "title1", "Your results", size=44, color=INK)
set_label(s, "score1", "%Course.ScorePercent%%", size=88, color=TEAL, bold=True)
find(s, "score1")["frame"] = F(100, 170, 500, 110)
set_label(s, "points1", "Points: %Course.ScorePoints%", color=BODY, size=22)
set_label(s, "passing1", "You need 80%% to pass.", color=MUTED, size=20)
find(s, "points1")["frame"] = F(104, 290, 500, 36)
find(s, "passing1")["frame"] = F(104, 330, 500, 36)
add(s, image("rs_badge", "badge.jpg", "An achievement ribbon with a leaf", F(760, 110, 420, 420), crop={"x": 0.28, "y": 0.0, "w": 0.44, "h": 1.0}, name="Badge"))
anim(find(s, "rs_badge"), "entrance", ZOOM, 700, 300)
for lid in ("passed", "failed"):
    for o in layer(s, lid)["objects"]:
        if o["kind"]["type"] == "button":
            o["frame"]["y"] = 600
            o["kind"]["fill"] = CORAL if ("finish" in o["id"] or "retry" in o["id"]) else TEAL
            o["states"] = [{"name": "hover", "fill": CORAL_DK if o["kind"]["fill"] == CORAL else TEAL_DK}]
set_label(s, "message1", "You passed. Great work, %learner_name%!", size=28, color=DEEP, bold=True)
set_label(s, "message2", "Not quite there yet. Review your answers or try again.", size=26, color=CORAL_DK, bold=True)
for mid in ("message1", "message2"):
    find(s, mid)["frame"] = F(104, 420, 640, 80)
for o in layer(s, "passed")["objects"]:
    if o["id"] == "passed_finish1":
        o["kind"]["label"] = "Continue  →"
layer_trigs = layer(s, "passed")["triggers"]
for t in layer_trigs:
    if t["when"].get("object") == "passed_finish1":
        t["action"] = jump("next")

# certificate restyle
s = "certificate1"
setup(s, CREAM, T_FADE, "Print or save your certificate.", master=False)
slide(s)["title"] = "Your certificate"
find(s, "border1")["kind"]["outline"] = {"color": SAGE, "width": 6, "dash": "solid"}
find(s, "learner1")["kind"]["text"] = "%learner_name%"
for oid, color in [("title1", DEEP), ("certifies1", MUTED), ("learner1", CORAL), ("completed1", MUTED), ("course1", INK), ("score1", BODY), ("date1", MUTED)]:
    find(s, oid)["kind"]["style"]["color"] = color
find(s, "learner1")["kind"]["style"]["size"] = 48
find(s, "learner1")["frame"] = F(128, 262, 1024, 70)
pr = find(s, "print1"); pr["kind"]["fill"] = TEAL; pr["frame"] = F(1000, 600, 150, 46)
next_button(s, 1000, 600, "Finish  →", w=150)
pr["frame"] = F(830, 600, 150, 46)

# ------------------------------------------------------------------ Finish
s = "finish"
setup(s, DEEP, T_FADE, "Well done! Pick one habit to try tomorrow. Small and steady beats big and rare.", master=False, duration=8000)
add(s, image("f_badge", "badge.jpg", "An achievement ribbon with a leaf", F(760, 120, 440, 440), crop={"x": 0.28, "y": 0.0, "w": 0.44, "h": 1.0}, name="Badge"))
anim(find(s, "f_badge"), "entrance", ZOOM, 800, 200)
add(s, obj("f_ring", "Ring", {"type": "shape", "shape": "ellipse", "fill": "#FFFFFF00", "outline": {"color": SAGE, "width": 3, "dash": "dot"}}, F(740, 100, 480, 480), decorative=True))
find(s, "f_ring")["rotation"] = 12
add(s, anim(eyebrow("f_eyebrow", "COURSE COMPLETE", 80, 200, SAGE), "entrance", fly("left"), 500, 300))
add(s, anim(text("f_title", "Well done,\n%learner_name%!", F(76, 230, 640, 160), 60, CREAM, True, "Title", ls=1.0), "entrance", fly("left"), 700, 400))
add(s, anim(text("f_sub", "Pick one habit to try tomorrow. Small and steady beats big and rare.", F(80, 400, 520, 70), 21, "#D5E3DC", False, "Subtitle", ls=1.35), "entrance", FADE, 600, 900))
add(s, obj("f_voice", "Closing narration", {"type": "audio", "src": "welldone.mp3", "autoplay": True, "captions": "welldone.vtt"}, F(80, 610, 36, 36)))
add(s, button("f_pause", "Pause voice", F(124, 610, 140, 36), TEAL))
trig(s, click("f_pause"), {"action": "pause_media", "object": "f_voice"})
add(s, anim(button("f_finish", "Finish course", F(80, 500, 220, 58), CORAL), "entrance", ZOOM, 500, 1200))
add(s, button("f_restart", "Start again", F(316, 500, 170, 58), TEAL))
trig(s, click("f_finish"), {"action": "exit_course"})
trig(s, click("f_restart"), jump("welcome"))
exit_anim = find(s, "f_sub")
exit_anim["exit"] = {"effect": {"type": "fade"}, "duration_ms": 600, "easing": "ease_in"}

# ------------------------------------------------------------------ Bonus (lightbox target)
s = "bonus"
setup(s, WHITE, T_FADE, "The 20-20-20 rule explained.", master=False)
add(s, image("bn_pic", "eyes.jpg", "A person resting their eyes by looking out of a window", F(0, 0, 560, 720), crop={"x": 0.25, "y": 0.0, "w": 0.43, "h": 1.0}, name="Eyes picture"))
add(s, eyebrow("bn_eyebrow", "BONUS", 620, 150))
add(s, heading("bn_title", "Why 20-20-20?", 618, 180, 600, 60, 44))
add(s, text("bn_list", "Every 20 minutes\nlook 20 feet (6 m) away\nfor 20 seconds", F(620, 260, 560, 150), 26, DEEP, True, "Rule",
            paras=[{"list": "number", "space_after": 10}, {"list": "number", "space_after": 10}, {"list": "number", "space_after": 10}]))
add(s, body("bn_body", "Focusing close up all day tires the small muscles in your eyes. Looking far away lets them relax. Blink a few times too.", 620, 430, 560, 110))
add(s, button("bn_close", "Back to the lesson", F(620, 570, 220, 52)))
trig(s, click("bn_close"), {"action": "close_lightbox"})

# ------------------------------------------------------------------ narration timed to the timeline
# Phrase start times come from the narration files (ffmpeg silencedetect), so each object enters
# as its words are spoken. Open a slide or layer timeline to see the stagger next to the audio bar.
def vtt(name, cues):
    lines = ["WEBVTT", ""]
    for a, b, t in cues:
        f = lambda ms: "00:%02d:%02d.%03d" % (ms // 60000, ms // 1000 % 60, ms % 1000)
        lines += [f"{f(a)} --> {f(b)}", t, ""]
    with open(os.path.join(ROOT, "media", name), "w") as fh:
        fh.write("\n".join(lines))

def voice(sid, oid, name, src, frame, cues, lid=None, autoplay=True):
    vtt(src.replace(".mp3", ".vtt"), cues)
    o = add(sid, obj(oid, name, {"type": "audio", "src": src, "autoplay": autoplay, "captions": src.replace(".mp3", ".vtt")}, frame), lid)
    # WCAG 1.4.2: a visible button pauses the autoplaying voice.
    btn = add(sid, button(f"{oid}_pause", "Pause voice", F(frame["x"], frame["y"], 132, 36), MUTED, name=f"Pause {name.lower()}"), lid)
    trig(sid, click(f"{oid}_pause"), {"action": "pause_media", "object": oid}, lid=lid)
    return o

def at(sid, oid, start, effect=None, ms=500, lid=None):
    o = find(sid, oid) if lid is None else next(x for x in layer(sid, lid)["objects"] if x["id"] == oid)
    anim(o, "entrance", effect or FADE, ms, start)
    return o

# Welcome: the subtitle waits for "In the next few minutes...", the button follows it.
for oid, ms in [("w_sub", 2400), ("w_start", 3200), ("w_chip", 3400), ("w_hint", 6400)]:
    find("welcome", oid)["timing"]["start_ms"] = ms

# Three small habits: each card rises as its name is spoken.
s = "habits"
slide(s)["duration_ms"] = 8000
voice(s, "h_voice", "Habits narration", "habits.mp3", F(850, 140, 36, 36),
      [(0, 1900, "Three small habits make a big difference."), (2700, 3300, "Move."), (4150, 4900, "Breathe."), (5950, 6700, "And connect.")])
for key, ms in [("move", 2700), ("breathe", 4150), ("connect", 5950)]:
    for oid in (f"h_card_{key}", f"h_pic_{key}", f"h_num_{key}", f"h_head_{key}", f"h_body_{key}"):
        find(s, oid)["timing"]["start_ms"] = ms
at(s, "h_hint", 6700)
habit_voice = {
    "move": ("move.mp3", 7000, [(0, 450, "Move."), (1100, 3000, "Sitting still is the problem, not sitting."), (3650, 6550, "Stand for calls, and change position every thirty minutes.")],
             ["Sitting still is the problem, not sitting.", "Stand for calls, and change position every 30 minutes."], [1100, 3650]),
    "breathe": ("breathe-tip.mp3", 5600, [(0, 500, "Breathe."), (1000, 3200, "Four counts in, four held, six out."), (3800, 5050, "Three rounds take about a minute.")],
                ["Four counts in, four held, six out.", "Three rounds take about a minute."], [1000, 3800]),
    "connect": ("connect.mp3", 4500, [(0, 450, "Connect."), (1030, 3360, "Swap one meeting a week for a walk and talk, outside.")],
                ["Swap one meeting a week for a walk and talk, outside.", "Most catch-ups don't need slides."], [1030, 3000]),
}
for key, (src, dur, cues, lines, starts) in habit_voice.items():
    lid = f"h_layer_{key}"
    L = layer(s, lid)
    L["duration_ms"] = dur
    L["pause_base_timeline"] = True
    L["objects"] = [o for o in L["objects"] if o["id"] != f"h_l_body_{key}"]
    voice(s, f"h_l_voice_{key}", "Habit narration", src, F(930, 130, 36, 36), cues, lid)
    at(s, f"h_l_eyebrow_{key}", 0, fly("right"), 400, lid)
    at(s, f"h_l_title_{key}", 150, fly("right"), 450, lid)
    for j, (line, ms) in enumerate(zip(lines, starts)):
        add(s, anim(body(f"h_l_line{j + 1}_{key}", line, 650, 268 + j * 104, 400, 96, 21, BODY, f"Point {j + 1}"), "entrance", fly("bottom"), 450, ms), lid)
    at(s, f"h_close_{key}", starts[-1] + 400, ZOOM, 350, lid)

# Desk: the markers pop in on "plus sign".
s = "desk"
voice(s, "d_voice", "Desk narration", "desk.mp3", F(1124, 36, 36, 36), [(0, 3200, "Tap each plus sign on the picture to see how to set up your desk.")])
for i, ms in [(1, 1100), (2, 1450), (3, 1800)]:
    find(s, f"d_m{i}")["timing"]["start_ms"] = ms

# Rest your eyes: three numbers, each on its words.
s = "eyes"
voice(s, "e_voice", "Eyes narration", "eyes.mp3", F(1124, 36, 36, 36), [(0, 3200, "Every twenty minutes, look about six metres away, for twenty seconds.")])
slide(s)["objects"] = [o for o in slide(s)["objects"] if o["id"] != "e_body"]
for j, (big, small, ms) in enumerate([("20 min", "Every", 0), ("6 m away", "Look", 900), ("20 s", "For", 2350)]):
    x = 660 + j * 184
    add(s, anim(shape(f"e_stat{j + 1}", F(x, 244, 172, 84), MIST, name=f"Rule part {j + 1}", label=f"{small}\n{big}",
                      lstyle=style(22, DEEP, True, align="center", valign="middle"), adjust=[0.18]), "entrance", ZOOM, 400, ms))

# A healthy day: the five times slide in while she says it.
s = "day"
voice(s, "dy_voice", "Day narration", "day.mp3", F(1124, 36, 36, 36), [(0, 2850, "Here's how the habits fit into a normal working day.")])
for i in range(1, 6):
    for oid in (f"timeline_point{i}", f"timeline_year{i}", f"timeline_title{i}"):
        anim(find(s, oid), "entrance", fly("bottom"), 400, 500 + i * 380)

# Results: each feedback layer talks, and its buttons wait for the words.
s = "results1"
voice(s, "rs_voice_pass", "Passed narration", "passed.mp3", F(1124, 36, 36, 36), [(0, 1300, "Great work! You passed.")], "passed")
voice(s, "rs_voice_fail", "Failed narration", "failed.mp3", F(1124, 36, 36, 36), [(0, 500, "Not quite."), (1130, 2850, "Review your answers, then try again.")], "failed")
for lid, plan in [("passed", [("message1", 0), ("passed_review1", 1500), ("passed_finish1", 1700)]),
                  ("failed", [("message2", 0), ("failed_review1", 1150), ("failed_retry1", 2200)])]:
    layer(s, lid)["duration_ms"] = max(layer(s, lid).get("duration_ms", 0), 4000)
    for oid, ms in plan:
        at(s, oid, ms, ZOOM if "1" in oid and "message" not in oid else FADE, 400, lid)

# ------------------------------------------------------------------ more features: bank draw, timeline control, timed exits, template
import copy

# Question bank with a random draw: three quick true/false questions, one is drawn after Question 9.
base_q = copy.deepcopy(slide("q3"))
bank = []
for n, (title, prompt, right, wrong) in enumerate([
        ("Bonus question", "True or false: a two-minute walk counts as a break.",
         "Right. Even two minutes on your feet resets your body and mind.", "It's true: a short walk is a real break."),
        ("Bonus question", "True or false: drinking water through the day helps you focus.",
         "Right. Even mild thirst makes it harder to concentrate.", "It's true: small sips through the day keep you sharp."),
        ("Bonus question", "True or false: looking far away now and then rests your eyes.",
         "Right. That's the 20-20-20 rule at work.", "It's true: distant focus lets your eye muscles relax.")], 1):
    q = json.loads(json.dumps(base_q).replace("q3_", "bank_"))
    q["id"], q["title"] = f"bank_q{n}", title
    q["question"]["prompt"] = prompt
    q["question"]["choices"] = [{"id": "true", "text": "True", "correct": True}, {"id": "false", "text": "False", "correct": False}]
    q["question"]["points"] = 10
    for o in q["objects"]:
        if o["id"] == "bank_eyebrow":
            o["kind"]["text"] = "BONUS  ·  DRAWN AT RANDOM"
    for L in q.get("layers", []):
        for o in L["objects"]:
            if o["id"] == "feedback_text1":
                o["kind"]["text"] = right
            elif o["id"] == "feedback_text2":
                o["kind"]["text"] = wrong
    q["notes"] = "Drawn at random from the Quick checks bank."
    bank.append(q)
C["banks"] = [{"id": "quick_checks", "title": "Quick checks", "slides": bank}]
quiz = next(sc for sc in C["scenes"] if sc["id"] == "sc_quiz")
at_q9 = next(i for i, x in enumerate(quiz["slides"]) if x["id"] == "q9")
quiz["slides"].insert(at_q9 + 1, {"id": "draw1", "title": "Random bonus question", "duration_ms": 5000,
                                   "draw": {"bank": "quick_checks", "count": 1},
                                   "notes": "This slide is replaced by one question picked at random from the Quick checks bank."})

# Pause and resume the breathing timeline (the Lottie, the cues and the leaf all wait).
s = "breathe"
add(s, button("b_hold", "Pause", F(272, 520, 96, 36), DEEP, name="Pause timeline button"))
add(s, button("b_go", "Resume", F(376, 520, 110, 36), DEEP, name="Resume timeline button"))
trig(s, click("b_hold"), {"action": "pause_timeline"})
trig(s, click("b_go"), {"action": "resume_timeline"})

# The "time's up" toast hides itself when its short layer timeline ends.
L = layer("eyes", "e_done")
L["duration_ms"] = 4000
L["hide_when_timeline_ends"] = True

# A hint that comes in, stays a few seconds and leaves with an exit animation.
s = "desk"
hint = add(s, anim(shape("d_pulse", F(70, 40, 260, 44), INK, name="Start hint", label="Tap a  +  to begin",
                         lstyle=style(17, WHITE, True, align="center", valign="middle"), adjust=[0.5]), "entrance", fly("top"), 400, 2000))
hint["timing"]["end_ms"] = 8000
slide(s)["duration_ms"] = 9000
hint["exit"] = {"effect": FADE, "duration_ms": 500, "easing": "ease_in"}

# Keyboard focus order on the welcome slide: Start first, then the narration controls.
for n, oid in enumerate(["w_start", "w_listen", "w_pause"], 1):
    find("welcome", oid)["focus_order"] = n

# Long flip-card text shrinks to fit its card.
for i in (1, 2, 3):
    find("myth", f"flip_card{i}")["kind"]["text"]["autofit"] = "shrink"

# Per-slide player rules: no seekbar on graded questions; Next waits for the welcome narration.
for sc in C["scenes"]:
    for x in sc["slides"]:
        if x.get("question") and x["id"].startswith("q"):
            x["player"] = {"show_seekbar": False}
slide("welcome")["player"] = {"next_requires": {"kind": "timeline_ends"}}

# A slide template authors can start new lesson pages from (New slide ▸ Your templates).
tpl = {"id": "tpl_lesson", "title": "Lesson page", "duration_ms": 5000, "background": CREAM, "master": "m_content",
       "transition": T_PUSH("right"), "objects": []}
C["templates"] = [tpl]
for o in [eyebrow("t_eyebrow", "LESSON  ·  TOPIC", 80, 110),
          heading("t_title", "Lesson title", 78, 140, 520, 120, 40),
          body("t_body", "One or two short sentences that explain the idea. Keep it under 30 words.", 80, 270, 500, 110),
          image("t_pic", "office.jpg", "Replace with a picture for this lesson", F(680, 64, 520, 560), crop={"x": 0.25, "y": 0.0, "w": 0.5, "h": 1.0}, name="Picture"),
          button("t_next", "Continue  →", F(80, 560, 180, 52))]:
    tpl["objects"].append(o)

# Custom font and background music.
C["fonts"] = [{"family": SERIF, "style": "regular", "src": "fonts/dm-serif-display-regular.ttf"}]
C["background_audio"] = {"src": "music.mp3", "volume": 0.12}

# ------------------------------------------------------------------ global restyle of inserted content
def walk(o):
    if isinstance(o, dict):
        for k, v in list(o.items()):
            if isinstance(v, str):
                if v.upper().startswith("#2F6FEB"):
                    o[k] = TEAL + v[7:]
                elif v.upper() == "#1F1F1F88":
                    o[k] = SAGE
                elif v.upper() == "#1F1F1F":
                    o[k] = INK
                elif v == "Open Sans":
                    o[k] = "Inter"
            else:
                walk(v)
    elif isinstance(o, list):
        for v in o:
            walk(v)
walk(C)
C["start_slide"] = "welcome"
with open(os.path.join(ROOT, "course.json"), "w") as f:
    json.dump(C, f, indent=1, ensure_ascii=False)
    f.write("\n")
print("slides:", sum(len(sc["slides"]) for sc in C["scenes"]))

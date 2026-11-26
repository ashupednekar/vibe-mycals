import { StatusBar } from "expo-status-bar";
import { Ionicons } from "@expo/vector-icons";
import { SafeAreaProvider, useSafeAreaInsets } from "react-native-safe-area-context";
import * as SecureStore from "expo-secure-store";
import * as WebBrowser from "expo-web-browser";
import * as ImagePicker from "expo-image-picker";
import React, { useCallback, useEffect, useState } from "react";
import {
  ActivityIndicator,
  Alert,
  Image,
  KeyboardAvoidingView,
  Modal,
  Platform,
  Pressable,
  RefreshControl,
  SafeAreaView,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from "react-native";

const API =
  ((globalThis as any).process?.env?.EXPO_PUBLIC_API_URL as string) ||
  "https://mycals.ashupednekar49.workers.dev";
const APP_SCHEME = "mycals://auth";
const SESSION_KEY = "mycals.session";

type AuthResponse = { token: string; user: User };

type User = {
  id: string;
  name: string;
  email: string;
  picture: string | null;
  sex: string;
  birth_year: number | null;
  height_cm: number | null;
  weight_kg: number | null;
  activity: string;
  goal: string;
  calorie_target: number;
  share_diary: boolean;
  household_id: string | null;
  onboarded: boolean;
};

type Entry = {
  id: string;
  meal: string;
  name: string;
  grams: number;
  kcal: number;
};

type Diary = {
  date: string;
  targets: { calorie: number };
  entries: Entry[];
  totals: { kcal: number };
};

type Food = {
  id: string;
  name: string;
  serving_grams: number | null;
  kcal100: number;
  is_public: boolean;
  shared: boolean;
};

type Member = {
  id: string;
  name: string;
  share_diary: boolean;
  calorie_target: number;
  kcal: number;
};

type Family = {
  household: { id: string; name: string; invite_code: string; owner_id: string } | null;
  members: Member[];
  is_owner: boolean;
};

type ScanItem = {
  name: string;
  grams: number;
  kcal: number;
  protein: number;
  carbs: number;
  fat: number;
};

type ScanResult = {
  scan_id?: string;
  name: string;
  description: string;
  items: ScanItem[];
  confidence: number;
};

type StoredScan = {
  id: string;
  name: string;
  description: string;
  confidence: number;
  items: ScanItem[];
  kcal: number;
  image_url: string | null;
  created_at: string;
};

type RefineResponse = {
  valid: boolean;
  reason: string;
  result: ScanResult;
};

type MemberDay = {
  name: string;
  calorie_target: number;
  totals: { kcal: number };
  entries: Entry[];
};

const C = {
  bg: "#0b1210",
  card: "#131e18",
  ink: "#edf3ea",
  muted: "#95a39a",
  line: "#25332b",
  green: "#7bd88f",
  greenDeep: "#1f7a4c",
  deep: "#0d3f2a",
  soft: "#1b2a22",
  lime: "#d8f26b",
  coral: "#ff8a65",
};

// Slightly larger type on Android; small screens/font metrics read tight.
const fs = (n: number) => Math.round(n * (Platform.OS === "android" ? 1.14 : 1));

async function request<T>(path: string, token?: string | null, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API}${path}`, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(token ? { authorization: `Bearer ${token}` } : {}),
      ...(init?.headers || {}),
    },
  });
  const text = await res.text();
  let data: any = null;
  try {
    data = text ? JSON.parse(text) : null;
  } catch {}
  if (!res.ok) throw new Error(data?.error || `Request failed (${res.status})`);
  return data as T;
}

const todayStr = () => {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
};

const shiftDate = (date: string, days: number) => {
  const d = new Date(`${date}T12:00:00`);
  d.setDate(d.getDate() + days);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
};

const prettyDate = (date: string) => {
  if (date === todayStr()) return "Today";
  if (date === shiftDate(todayStr(), -1)) return "Yesterday";
  return new Date(`${date}T12:00:00`).toLocaleDateString(undefined, {
    weekday: "short",
    month: "short",
    day: "numeric",
  });
};

const initials = (name: string) =>
  name
    .split(" ")
    .map((w) => w[0])
    .filter(Boolean)
    .slice(0, 2)
    .join("")
    .toUpperCase();

const msg = (e: unknown) => (e instanceof Error ? e.message : String(e));

// ------------------------------------------------------------------ ui kit

function Btn({
  label,
  onPress,
  variant = "primary",
  disabled,
  small,
}: {
  label: string;
  onPress: () => void;
  variant?: "primary" | "ghost" | "outline" | "lime";
  disabled?: boolean;
  small?: boolean;
}) {
  const style =
    variant === "primary"
      ? s.btnPrimary
      : variant === "ghost"
      ? s.btnGhost
      : variant === "lime"
      ? s.btnLime
      : s.btnOutline;
  const textStyle =
    variant === "primary"
      ? s.btnTextLight
      : variant === "lime"
      ? { ...s.btnText, color: "#29360a" }
      : variant === "ghost"
      ? { ...s.btnText, color: C.green }
      : s.btnText;
  return (
    <Pressable
      onPress={onPress}
      disabled={disabled}
      style={({ pressed }) => [
        s.btn,
        small && s.btnSmall,
        style,
        (pressed || disabled) && { opacity: 0.75 },
      ]}
    >
      <Text style={[s.btnText, textStyle]}>{label}</Text>
    </Pressable>
  );
}

function Avatar({ name, size = 38, tone = "neutral" }: { name: string; size?: number; tone?: "neutral" | "green" }) {
  return (
    <View
      style={{
        width: size,
        height: size,
        borderRadius: size / 2,
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: tone === "green" ? C.green : "#26342c",
      }}
    >
      <Text style={{ fontSize: size / 2.9, fontWeight: "800", color: tone === "green" ? C.lime : "#cfe0d5" }}>
        {initials(name)}
      </Text>
    </View>
  );
}

function Field({
  label,
  value,
  onChange,
  numeric,
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (v: string) => void;
  numeric?: boolean;
  placeholder?: string;
}) {
  return (
    <View style={{ flex: 1, gap: 5 }}>
      <Text style={s.fieldLabel}>{label}</Text>
      <TextInput
        value={value}
        onChangeText={onChange}
        keyboardType={numeric ? "numeric" : "default"}
        placeholder={placeholder}
        placeholderTextColor="#6f7f73"
        style={s.input}
      />
    </View>
  );
}

function Banner({ text }: { text: string }) {
  if (!text) return null;
  return (
    <View style={s.banner}>
      <Text style={{ color: "#ffb4a2", fontSize: fs(12.5), fontWeight: "600" }}>{text}</Text>
    </View>
  );
}

function Progress({ value, tone = C.green }: { value: number; tone?: string }) {
  return (
    <View style={s.track}>
      <View style={[s.fill, { width: `${Math.min(100, Math.max(0, value))}%` as any, backgroundColor: tone }]} />
    </View>
  );
}

// ------------------------------------------------------------------ app

export default function App() {
  const [booting, setBooting] = useState(true);
  const [token, setToken] = useState<string | null>(null);
  const [user, setUser] = useState<User | null>(null);

  useEffect(() => {
    (async () => {
      try {
        const saved = await SecureStore.getItemAsync(SESSION_KEY);
        if (saved) {
          const me = await request<User>("/api/me", saved);
          setToken(saved);
          setUser(me);
        }
      } catch {
        await SecureStore.deleteItemAsync(SESSION_KEY).catch(() => {});
      }
      setBooting(false);
    })();
  }, []);

  const signIn = async (t: string) => {
    const clean = t.trim();
    if (!clean) throw new Error("Empty session token");
    // Verify before persisting so a bad/stale token can never get stuck in the keychain.
    const me = await request<User>("/api/me", clean);
    await SecureStore.setItemAsync(SESSION_KEY, clean);
    setToken(clean);
    setUser(me);
  };

  const startGoogle = async () => {
    await SecureStore.deleteItemAsync(SESSION_KEY).catch(() => {});
    const attempt = `${Date.now().toString(36)}${Math.random().toString(36).slice(2, 10)}`;
    const result = await WebBrowser.openAuthSessionAsync(
      `${API}/api/auth/google/start?mobile=1&attempt=${attempt}`,
      APP_SCHEME
    );
    const waitMs = result.type === "success" || result.type === "opened" ? 30000 : 4000;
    const deadline = Date.now() + waitMs;
    while (Date.now() < deadline) {
      const pending = await request<{ code: string | null }>(
        `/api/auth/mobile/pending?attempt=${encodeURIComponent(attempt)}`,
        null
      );
      if (pending.code) {
        const res = await request<AuthResponse>("/api/auth/mobile/exchange", null, {
          method: "POST",
          body: JSON.stringify({ code: pending.code }),
        });
        await signIn(res.token);
        return;
      }
      await new Promise((resolve) => setTimeout(resolve, 800));
    }
    throw new Error(
      result.type === "success" || result.type === "opened"
        ? "Sign-in timed out. Try again."
        : `Sign-in did not complete (${result.type}). Try again.`
    );
  };

  const signOut = async () => {
    await SecureStore.deleteItemAsync(SESSION_KEY).catch(() => {});
    setToken(null);
    setUser(null);
  };

  if (booting) {
    return (
      <SafeAreaView style={s.center}>
        <StatusBar style="light" />
        <View style={s.markLarge}>
          <Text style={s.markLargeText}>m</Text>
        </View>
        <ActivityIndicator color={C.green} style={{ marginTop: 18 }} />
      </SafeAreaView>
    );
  }

  return (
    <SafeAreaProvider>
      <View style={{ flex: 1, backgroundColor: C.bg }}>
        <StatusBar style="light" />
        {!token || !user ? (
          <Login onGoogle={startGoogle} />
        ) : !user.onboarded ? (
          <Onboarding token={token} onDone={setUser} />
        ) : (
          <Main token={token} user={user} onUser={setUser} onSignOut={signOut} />
        )}
      </View>
    </SafeAreaProvider>
  );
}

// ------------------------------------------------------------------ login

function Login({ onGoogle }: { onGoogle: () => Promise<void> }) {
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  const google = async () => {
    setError("");
    setBusy(true);
    try {
      await onGoogle();
    } catch (e) {
      setError(msg(e));
    }
    setBusy(false);
  };

  return (
    <SafeAreaView style={{ flex: 1, backgroundColor: C.bg }}>
      <ScrollView contentContainerStyle={s.login}>
        <View style={s.brandRow}>
          <View style={s.mark}>
            <Text style={s.markText}>m</Text>
          </View>
          <Text style={s.brand}>
            my<Text style={{ color: C.green, fontWeight: "700" }}>cals</Text>
          </Text>
        </View>
        <Text style={s.h1}>
          Count calories.{"\n"}No <Text style={{ color: C.green, fontStyle: "italic" }}>nonsense.</Text>
        </Text>
        <Text style={s.lede}>
          A no-nonsense calorie tracker for the whole family. Log a meal in a tap, scan a plate, and
          see how everyone is doing.
        </Text>
        <View style={s.chips}>
          {["🔐 Google sign-in", "👨‍👩‍👧 family diary", "✨ photo scan"].map((t) => (
            <View key={t} style={s.chip}>
              <Text style={s.chipText}>{t}</Text>
            </View>
          ))}
        </View>
        <View style={{ gap: 10, marginTop: 6 }}>
          <Btn
            label={busy ? "Opening Google…" : "Continue with Google"}
            onPress={google}
            variant="outline"
            disabled={busy}
          />
        </View>
        <Banner text={error} />
        <Pressable
          onPress={async () => {
            await SecureStore.deleteItemAsync(SESSION_KEY).catch(() => {});
            setError("Saved session cleared. Try again.");
          }}
        >
          <Text style={[s.tiny, { textAlign: "center", textDecorationLine: "underline" }]}>
            Clear saved session
          </Text>
        </Pressable>
        <View style={{ flex: 1 }} />
        <Text style={[s.tiny, { textAlign: "center" }]}>
          Calories only. Family optional. Scans stay in your history.
        </Text>
      </ScrollView>
    </SafeAreaView>
  );
}

// ------------------------------------------------------------------ onboarding

function Onboarding({ token, onDone }: { token: string; onDone: (u: User) => void }) {
  const [name, setName] = useState("");
  const [sex, setSex] = useState("female");
  const [birthYear, setBirthYear] = useState("1995");
  const [height, setHeight] = useState("170");
  const [weight, setWeight] = useState("70");
  const [activity, setActivity] = useState("moderate");
  const [goal, setGoal] = useState("lose");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    request<User>("/api/me", token)
      .then((u) => {
        if (u.name) setName(u.name);
        setSex(u.sex || "female");
        setBirthYear(String(u.birth_year || 1995));
        setHeight(String(u.height_cm || 170));
        setWeight(String(u.weight_kg || 70));
        setActivity(u.activity || "moderate");
        setGoal(u.goal || "lose");
      })
      .catch(() => {});
  }, [token]);

  const save = async () => {
    setBusy(true);
    setError("");
    try {
      const u = await request<User>("/api/profile", token, {
        method: "PUT",
        body: JSON.stringify({
          name,
          sex,
          birth_year: Number(birthYear),
          height_cm: Number(height),
          weight_kg: Number(weight),
          activity,
          goal,
          share_diary: true,
          onboarded: true,
        }),
      });
      onDone(u);
    } catch (e) {
      setError(msg(e));
    }
    setBusy(false);
  };

  return (
    <SafeAreaView style={{ flex: 1, backgroundColor: C.bg }}>
      <KeyboardAvoidingView behavior={Platform.OS === "ios" ? "padding" : undefined} style={{ flex: 1 }}>
        <ScrollView contentContainerStyle={s.onboard}>
          <Text style={s.eyebrow}>Let's set your baseline</Text>
          <Text style={s.h2}>A few numbers, then you're in.</Text>
          <Text style={s.lede}>
            We use Mifflin-St Jeor to set a starting calorie target. You can change it later.
          </Text>
          <View style={{ gap: 12, marginTop: 8 }}>
            <Field label="Your name" value={name} onChange={setName} placeholder="Alex" />
            <View style={{ flexDirection: "row", gap: 10 }}>
              <View style={{ flex: 1, gap: 5 }}>
                <Text style={s.fieldLabel}>Sex</Text>
                <View style={s.row}>
                  {["female", "male"].map((v) => (
                    <Pressable key={v} onPress={() => setSex(v)} style={[s.pill, sex === v && s.pillActive]}>
                      <Text style={[s.pillText, sex === v && s.pillTextActive]}>{v}</Text>
                    </Pressable>
                  ))}
                </View>
              </View>
              <Field label="Birth year" value={birthYear} onChange={setBirthYear} numeric />
            </View>
            <View style={{ flexDirection: "row", gap: 10 }}>
              <Field label="Height (cm)" value={height} onChange={setHeight} numeric />
              <Field label="Weight (kg)" value={weight} onChange={setWeight} numeric />
            </View>
            <Text style={s.fieldLabel}>Activity</Text>
            <View style={s.wrap}>
              {[
                ["sedentary", "Sedentary"],
                ["light", "Light"],
                ["moderate", "Moderate"],
                ["active", "Very active"],
              ].map(([v, l]) => (
                <Pressable key={v} onPress={() => setActivity(v)} style={[s.pill, activity === v && s.pillActive]}>
                  <Text style={[s.pillText, activity === v && s.pillTextActive]}>{l}</Text>
                </Pressable>
              ))}
            </View>
            <Text style={s.fieldLabel}>Goal</Text>
            <View style={s.wrap}>
              {[
                ["lose", "Lose"],
                ["maintain", "Maintain"],
                ["gain", "Gain"],
              ].map(([v, l]) => (
                <Pressable key={v} onPress={() => setGoal(v)} style={[s.pill, goal === v && s.pillActive]}>
                  <Text style={[s.pillText, goal === v && s.pillTextActive]}>{l}</Text>
                </Pressable>
              ))}
            </View>
          </View>
          <Banner text={error} />
          <Btn label={busy ? "Saving…" : "Start tracking"} onPress={save} disabled={busy} />
        </ScrollView>
      </KeyboardAvoidingView>
    </SafeAreaView>
  );
}

// ------------------------------------------------------------------ main

type Tab = "today" | "scan" | "family" | "you";

function Main({
  token,
  user,
  onUser,
  onSignOut,
}: {
  token: string;
  user: User;
  onUser: (u: User) => void;
  onSignOut: () => void;
}) {
  const [tab, setTab] = useState<Tab>("today");
  const insets = useSafeAreaInsets();
  return (
    <View style={{ flex: 1, backgroundColor: C.bg }}>
      <View style={{ flex: 1, paddingTop: insets.top }}>
        {tab === "today" && <Today token={token} user={user} onScan={() => setTab("scan")} />}
        {tab === "scan" && <Scan token={token} onDone={() => setTab("today")} />}
        {tab === "family" && <Family token={token} user={user} />}
        {tab === "you" && <You token={token} user={user} onUser={onUser} onSignOut={onSignOut} />}
      </View>
      <View style={[s.tabbar, { paddingBottom: Math.max(insets.bottom, 8) }]}>
        {(
          [
            ["today", "home-outline", "Today"],
            ["scan", "camera-outline", "Scan"],
            ["family", "people-outline", "Family"],
            ["you", "person-outline", "You"],
          ] as [Tab, string, string][]
        ).map(([key, icon, label]) => (
          <Pressable key={key} onPress={() => setTab(key)} style={[s.tab, tab === key && s.tabActive]}>
            <Ionicons name={icon as any} size={19} color={tab === key ? C.green : C.muted} />
            <Text style={[s.tabText, tab === key && { color: C.green, fontWeight: "700" }]}>{label}</Text>
          </Pressable>
        ))}
      </View>
    </View>
  );
}

// ------------------------------------------------------------------ today

function Today({ token, user, onScan }: { token: string; user: User; onScan: () => void }) {
  const [date, setDate] = useState(todayStr());
  const [diary, setDiary] = useState<Diary | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState("");
  const [sheet, setSheet] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setError("");
      const d = await request<Diary>(`/api/diary?date=${date}`, token);
      setDiary(d);
    } catch (e) {
      setError(msg(e));
    }
    setLoading(false);
    setRefreshing(false);
  }, [date, token]);

  useEffect(() => {
    setLoading(true);
    load();
  }, [load]);

  const kcal = diary?.totals.kcal ?? 0;
  const target = diary?.targets.calorie ?? user.calorie_target;
  const left = Math.round(target - kcal);
  const warning = kcal >= target * 0.8;
  const meals: [string, string, string][] = [
    ["breakfast", "Breakfast", "sunny-outline"],
    ["lunch", "Lunch", "restaurant-outline"],
    ["dinner", "Dinner", "moon-outline"],
    ["snacks", "Snacks", "nutrition-outline"],
  ];

  return (
    <ScrollView
      contentContainerStyle={s.page}
      refreshControl={<RefreshControl refreshing={refreshing} onRefresh={() => { setRefreshing(true); load(); }} tintColor={C.green} />}
    >
      <View style={s.topRow}>
        <View>
          <Text style={s.eyebrow}>Your diary</Text>
          <Text style={s.dateTitle}>{prettyDate(date)}</Text>
        </View>
        <View style={{ alignItems: "flex-end", gap: 8 }}>
          <Avatar name={user.name} tone="green" />
          <View style={s.dateNav}>
            <Pressable onPress={() => setDate(shiftDate(date, -1))} style={s.chev}>
              <Text style={s.chevText}>‹</Text>
            </Pressable>
            <Pressable onPress={() => setDate(todayStr())} style={s.todayBtn}>
              <Text style={s.todayBtnText}>Today</Text>
            </Pressable>
            <Pressable
              onPress={() => date < todayStr() && setDate(shiftDate(date, 1))}
              style={[s.chev, date >= todayStr() && { opacity: 0.35 }]}
            >
              <Text style={s.chevText}>›</Text>
            </Pressable>
          </View>
        </View>
      </View>

      <Banner text={error} />

      <View style={s.card}>
        <View style={s.rowBetween}>
          <Text style={s.cardTitle}>Calories</Text>
          <Text style={s.mutedSmall}>{target} goal</Text>
        </View>
        <View style={s.calRow}>
          <Text style={[s.calBig, warning && { color: C.coral }]}>{Math.round(kcal)}</Text>
          <View style={{ flex: 1 }}>
            <Text style={s.mutedSmall}>of {target} kcal</Text>
            <Text style={[s.remaining, (left < 0 || warning) && { color: C.coral }]}>
              {left >= 0 ? `${left} left` : `${Math.abs(left)} over`}
            </Text>
          </View>
        </View>
        <Progress value={target ? (kcal / target) * 100 : 0} tone={warning ? C.coral : C.green} />
        <Pressable onPress={onScan} style={s.scanCta}>
          <Ionicons name="camera-outline" size={18} color={C.green} />
          <Text style={s.scanCtaText}>Scan a plate or label</Text>
          <Text style={{ marginLeft: "auto", color: C.green, fontWeight: "700" }}>→</Text>
        </Pressable>
      </View>

      {loading && !diary ? (
        <ActivityIndicator color={C.green} style={{ marginTop: 30 }} />
      ) : (
        meals.map(([key, label, emoji]) => {
          const items = diary?.entries.filter((e) => e.meal === key) ?? [];
          const total = items.reduce((n, e) => n + e.kcal, 0);
          return (
            <View key={key} style={s.card}>
              <View style={s.mealHead}>
                <View style={{ flexDirection: "row", alignItems: "center", gap: 7 }}>
                  <Ionicons name={emoji as any} size={18} color={C.green} />
                  <Text style={s.mealTitle}>{label}</Text>
                </View>
                <View style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
                  <Text style={s.mutedSmall}>{Math.round(total)} kcal</Text>
                  <Pressable onPress={() => setSheet(key)} style={s.addBtn}>
                    <Text style={s.addBtnText}>+</Text>
                  </Pressable>
                </View>
              </View>
              {items.length === 0 ? (
                <Pressable onPress={() => setSheet(key)}>
                  <Text style={s.empty}>Nothing logged yet — tap to add</Text>
                </Pressable>
              ) : (
                items.map((e) => (
                  <View key={e.id} style={s.entry}>
                    <View style={{ flex: 1 }}>
                      <Text style={s.entryName} numberOfLines={1}>
                        {e.name}
                      </Text>
                      <Text style={s.mutedSmall}>{Math.round(e.grams)} g</Text>
                    </View>
                    <Text style={s.entryKcal}>{Math.round(e.kcal)}</Text>
                  </View>
                ))
              )}
            </View>
          );
        })
      )}

      {sheet && (
        <FoodSheet
          token={token}
          date={date}
          meal={sheet}
          onClose={() => setSheet(null)}
          onAdded={() => {
            setSheet(null);
            load();
          }}
        />
      )}
    </ScrollView>
  );
}

function FoodSheet({
  token,
  date,
  meal,
  onClose,
  onAdded,
}: {
  token: string;
  date: string;
  meal: string;
  onClose: () => void;
  onAdded: () => void;
}) {
  const [q, setQ] = useState("");
  const [foods, setFoods] = useState<Food[]>([]);
  const [selected, setSelected] = useState<Food | null>(null);
  const [grams, setGrams] = useState("100");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    const t = setTimeout(async () => {
      try {
        const list = await request<Food[]>(`/api/foods?q=${encodeURIComponent(q)}`, token);
        setFoods(list);
      } catch (e) {
        setError(msg(e));
      }
    }, 220);
    return () => clearTimeout(t);
  }, [q, token]);

  const amount = Number(grams) || 0;
  const preview = selected ? Math.round((selected.kcal100 * amount) / 100) : 0;

  const add = async () => {
    if (!selected) return;
    setBusy(true);
    setError("");
    try {
      await request("/api/diary/entries", token, {
        method: "POST",
        body: JSON.stringify({
          date,
          meal,
          food_id: selected.id,
          name: selected.name,
          grams: amount,
          kcal: 0,
          protein: 0,
          carbs: 0,
          fat: 0,
        }),
      });
      onAdded();
    } catch (e) {
      setError(msg(e));
      setBusy(false);
    }
  };

  return (
    <Modal visible animationType="slide" presentationStyle="pageSheet" onRequestClose={onClose}>
      <SafeAreaView style={{ flex: 1, backgroundColor: C.bg }}>
        <ScrollView contentContainerStyle={{ padding: 18, gap: 14 }}>
          <View style={s.rowBetween}>
            <View>
              <Text style={s.eyebrow}>Add to {meal}</Text>
              <Text style={s.h2}>What did you eat?</Text>
            </View>
            <Pressable onPress={onClose} style={s.closeBtn}>
              <Text style={{ fontSize: fs(22), color: C.muted }}>×</Text>
            </Pressable>
          </View>

          {selected ? (
            <View style={s.card}>
              <Text style={s.selName}>{selected.name}</Text>
              <Text style={s.mutedSmall}>
                {Math.round(selected.kcal100)} kcal / 100 g{selected.shared ? " · family" : ""}
              </Text>
              <View style={{ flexDirection: "row", alignItems: "flex-end", gap: 14, marginTop: 14 }}>
                <View style={{ flex: 1 }}>
                  <Text style={s.fieldLabel}>Portion (g)</Text>
                  <TextInput
                    value={grams}
                    onChangeText={setGrams}
                    keyboardType="numeric"
                    style={s.input}
                  />
                </View>
                <Text style={s.previewKcal}>{preview} kcal</Text>
              </View>
              <View style={{ flexDirection: "row", gap: 8, marginTop: 14 }}>
                <Btn label="Add" onPress={add} disabled={busy || amount <= 0} />
                <Btn label="Back" onPress={() => setSelected(null)} variant="outline" small />
              </View>
            </View>
          ) : (
            <>
              <TextInput
                value={q}
                onChangeText={setQ}
                placeholder="Search chicken, oats, dal…"
                placeholderTextColor="#6f7f73"
                style={s.input}
              />
              <Banner text={error} />
              <View style={{ gap: 2 }}>
                {foods.map((f) => (
                  <Pressable
                    key={f.id}
                    onPress={() => {
                      setSelected(f);
                      setGrams(String(f.serving_grams || 100));
                    }}
                    style={s.foodRow}
                  >
                    <View style={{ flex: 1 }}>
                      <Text style={s.foodName}>{f.name}</Text>
                      <Text style={s.mutedSmall}>
                        {f.is_public ? "public" : "custom"} · {Math.round(f.kcal100)} kcal/100 g
                      </Text>
                    </View>
                    <Text style={s.foodKcal}>{Math.round(f.kcal100)}</Text>
                  </Pressable>
                ))}
              </View>
            </>
          )}
        </ScrollView>
      </SafeAreaView>
    </Modal>
  );
}

// ------------------------------------------------------------------ scan

function Scan({ token, onDone }: { token: string; onDone: () => void }) {
  const [asset, setAsset] = useState<ImagePicker.ImagePickerAsset | null>(null);
  const [result, setResult] = useState<ScanResult | null>(null);
  const [meal, setMeal] = useState("lunch");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [correction, setCorrection] = useState("");
  const [refineError, setRefineError] = useState("");
  const [refining, setRefining] = useState(false);
  const [history, setHistory] = useState<StoredScan[]>([]);

  const loadHistory = async () => {
    try {
      setHistory(await request<StoredScan[]>("/api/scans", token));
    } catch {}
  };

  useEffect(() => {
    loadHistory();
  }, [token]);

  const pick = async (camera: boolean) => {
    setError("");
    const perm = camera
      ? await ImagePicker.requestCameraPermissionsAsync()
      : await ImagePicker.requestMediaLibraryPermissionsAsync();
    if (!perm.granted) {
      Alert.alert("Permission needed", "Allow camera or photo access to scan a meal.");
      return;
    }
    const res = camera
      ? await ImagePicker.launchCameraAsync({ mediaTypes: ["images"], quality: 0.6, base64: true })
      : await ImagePicker.launchImageLibraryAsync({ mediaTypes: ["images"], quality: 0.6, base64: true });
    if (!res.canceled && res.assets[0]) {
      setAsset(res.assets[0]);
      setResult(null);
    }
  };

  const analyze = async () => {
    if (!asset?.base64) {
      setError("This photo could not be read. Try another.");
      return;
    }
    setBusy(true);
    setError("");
    try {
      const r = await request<ScanResult>("/api/scan", token, {
        method: "POST",
        body: JSON.stringify({
          image_base64: asset.base64,
          mime_type: asset.mimeType || "image/jpeg",
        }),
      });
      setResult(r);
      loadHistory();
    } catch (e) {
      setError(msg(e));
    }
    setBusy(false);
  };

  const refine = async () => {
    if (!result || !correction.trim()) return;
    setRefining(true);
    setRefineError("");
    try {
      const r = await request<RefineResponse>("/api/scan/refine", token, {
        method: "POST",
        body: JSON.stringify({
          scan_id: result.scan_id,
          prompt: correction.trim(),
          result,
        }),
      });
      if (r.valid) {
        setResult(r.result);
        setCorrection("");
        loadHistory();
      } else {
        setRefineError(r.reason);
      }
    } catch (e) {
      setRefineError(msg(e));
    }
    setRefining(false);
  };

  const add = async () => {
    if (!result) return;
    setBusy(true);
    setError("");
    try {
      for (const item of result.items) {
        await request("/api/diary/entries", token, {
          method: "POST",
          body: JSON.stringify({
            date: todayStr(),
            meal,
            name: result.items.length > 1 ? `${result.name} · ${item.name}` : item.name,
            grams: item.grams,
            kcal: item.kcal,
            protein: item.protein,
            carbs: item.carbs,
            fat: item.fat,
          }),
        });
      }
      onDone();
    } catch (e) {
      setError(msg(e));
      setBusy(false);
    }
  };

  const total = result?.items.reduce((n, i) => n + i.kcal, 0) ?? 0;

  return (
    <ScrollView contentContainerStyle={s.page}>
      <Text style={s.eyebrow}>Photo scan</Text>
      <Text style={s.dateTitle}>Scan a plate</Text>
      <Text style={s.lede}>Point at a meal or a nutrition label. We estimate the food and portion.</Text>

      {asset ? (
        <View style={s.card}>
          <Image source={{ uri: asset.uri }} style={s.preview} />
          <View style={{ flexDirection: "row", gap: 8, marginTop: 12 }}>
            <Btn label="Retake" onPress={() => pick(true)} variant="outline" small />
            <Btn
              label={busy ? "Reading…" : "Detect calories"}
              onPress={analyze}
              disabled={busy}
            />
          </View>
        </View>
      ) : (
        <View style={s.card}>
          <Pressable onPress={() => pick(true)} style={s.scanDrop}>
            <Ionicons name="camera-outline" size={44} color={C.green} />
            <Text style={{ fontWeight: "700" }}>Take a photo</Text>
            <Text style={s.mutedSmall}>or choose one from your library</Text>
          </Pressable>
          <View style={{ flexDirection: "row", gap: 8, marginTop: 12 }}>
            <Btn label="Camera" onPress={() => pick(true)} />
            <Btn label="Library" onPress={() => pick(false)} variant="outline" />
          </View>
        </View>
      )}

      <Banner text={error} />

      {result && (
        <View style={s.card}>
          <View style={s.rowBetween}>
            <View style={{ flex: 1 }}>
              <Text style={s.selName}>{result.name}</Text>
              <Text style={s.mutedSmall}>{result.description}</Text>
            </View>
            <Text style={s.previewKcal}>{Math.round(total)}</Text>
          </View>
          <View style={{ marginTop: 12 }}>
            {result.items.map((item, i) => (
              <View key={i} style={s.entry}>
                <View style={{ flex: 1 }}>
                  <Text style={s.entryName}>{item.name}</Text>
                  <Text style={s.mutedSmall}>{Math.round(item.grams)} g</Text>
                </View>
                <Text style={s.entryKcal}>{Math.round(item.kcal)}</Text>
              </View>
            ))}
          </View>
          <View style={{ marginTop: 14 }}>
            <Text style={s.fieldLabel}>Something off? Adjust it</Text>
            <View style={{ flexDirection: "row", gap: 8, alignItems: "center", marginTop: 6 }}>
              <TextInput
                value={correction}
                onChangeText={setCorrection}
                placeholder={'e.g. "only half the rice"'}
                placeholderTextColor="#6f7f73"
                style={[s.input, { flex: 1 }]}
              />
              <Btn
                label={refining ? "…" : "Recalculate"}
                onPress={refine}
                disabled={refining || !correction.trim()}
                small
              />
            </View>
            <Text style={s.tiny}>Only portion corrections are accepted.</Text>
            {refineError ? (
              <Text style={{ color: C.coral, fontSize: fs(12), marginTop: 4 }}>{refineError}</Text>
            ) : null}
          </View>
          <Text style={[s.fieldLabel, { marginTop: 14 }]}>Add to</Text>
          <View style={s.wrap}>
            {[
              ["breakfast", "Breakfast"],
              ["lunch", "Lunch"],
              ["dinner", "Dinner"],
              ["snacks", "Snacks"],
            ].map(([v, l]) => (
              <Pressable key={v} onPress={() => setMeal(v)} style={[s.pill, meal === v && s.pillActive]}>
                <Text style={[s.pillText, meal === v && s.pillTextActive]}>{l}</Text>
              </Pressable>
            ))}
          </View>
          <View style={{ marginTop: 12 }}>
            <Btn
              label={busy ? "Adding…" : `Add ${result.items.length} to diary`}
              onPress={add}
              disabled={busy}
            />
          </View>
        </View>
      )}
      <Text style={s.tiny}>AI estimates. Check the numbers before logging.</Text>

      {history.length > 0 && (
        <View style={{ gap: 10, marginTop: 4 }}>
          <Text style={s.cardTitle}>Recent scans</Text>
          {history.slice(0, 8).map((h) => (
            <Pressable
              key={h.id}
              style={[s.card, { flexDirection: "row", alignItems: "center", gap: 10, padding: 10 }]}
              onPress={() =>
                setResult({
                  scan_id: h.id,
                  name: h.name,
                  description: h.description,
                  items: h.items,
                  confidence: h.confidence,
                })
              }
            >
              {h.image_url ? (
                <Image
                  source={{
                    uri: `${API}${h.image_url}`,
                    headers: { authorization: `Bearer ${token}` },
                  }}
                  style={{ width: 58, height: 58, borderRadius: 12 }}
                />
              ) : (
                <View style={{ width: 58, height: 58, borderRadius: 12, backgroundColor: C.line }} />
              )}
              <View style={{ flex: 1 }}>
                <Text style={s.entryName} numberOfLines={1}>{h.name}</Text>
                <Text style={s.mutedSmall} numberOfLines={2}>{h.description}</Text>
                <Text style={s.mutedSmall}>
                  {Math.round(h.kcal)} kcal · {h.created_at.slice(0, 10)}
                </Text>
              </View>
            </Pressable>
          ))}
        </View>
      )}
    </ScrollView>
  );
}

// ------------------------------------------------------------------ family

function Family({ token, user }: { token: string; user: User }) {
  const [family, setFamily] = useState<Family | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [joinCode, setJoinCode] = useState("");
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [member, setMember] = useState<Member | null>(null);

  const load = useCallback(async () => {
    try {
      setError("");
      setFamily(await request<Family>(`/api/family?date=${todayStr()}`, token));
    } catch (e) {
      setError(msg(e));
    }
    setLoading(false);
  }, [token]);

  useEffect(() => {
    load();
  }, [load]);

  const create = async () => {
    setBusy(true);
    setError("");
    try {
      const f = await request<Family>("/api/family", token, {
        method: "POST",
        body: JSON.stringify({ name }),
      });
      setFamily(f);
    } catch (e) {
      setError(msg(e));
    }
    setBusy(false);
  };

  const join = async () => {
    setBusy(true);
    setError("");
    try {
      const f = await request<Family>("/api/family/join", token, {
        method: "POST",
        body: JSON.stringify({ code: joinCode }),
      });
      setFamily(f);
    } catch (e) {
      setError(msg(e));
    }
    setBusy(false);
  };

  const leave = async () => {
    Alert.alert("Leave family?", "You'll stop seeing everyone's diary.", [
      { text: "Cancel", style: "cancel" },
      {
        text: "Leave",
        style: "destructive",
        onPress: async () => {
          try {
            await request("/api/family/leave", token, { method: "POST", body: "{}" });
            setFamily({ household: null, members: [], is_owner: false });
          } catch (e) {
            setError(msg(e));
          }
        },
      },
    ]);
  };

  return (
    <ScrollView contentContainerStyle={s.page}>
      <Text style={s.eyebrow}>Family sharing</Text>
      <Text style={s.dateTitle}>Your people</Text>
      <Banner text={error} />

      {loading ? (
        <ActivityIndicator color={C.green} style={{ marginTop: 30 }} />
      ) : family?.household ? (
        <>
          <View style={[s.card, { backgroundColor: "#1a2620" }]}>
            <Text style={s.eyebrow}>Invite code</Text>
            <View style={{ flexDirection: "row", alignItems: "center", gap: 12, marginTop: 4 }}>
              <Text style={s.invite}>{family.household.invite_code}</Text>
              <View style={{ flex: 1 }} />
              <Text style={s.mutedSmall}>{family.household.name}</Text>
            </View>
            <Text style={[s.mutedSmall, { marginTop: 6 }]}>Share this with up to 7 people.</Text>
          </View>

          <Text style={[s.cardTitle, { marginTop: 4 }]}>Today · {prettyDate(todayStr())}</Text>
          {family.members.map((m) => {
            const warn = m.kcal >= m.calorie_target * 0.8;
            return (
              <Pressable key={m.id} onPress={() => setMember(m)} style={s.card}>
                <View style={{ flexDirection: "row", alignItems: "center", gap: 10 }}>
                  <Avatar name={m.name} tone={m.id === user.id ? "green" : "neutral"} />
                  <View style={{ flex: 1 }}>
                    <Text style={s.memberName}>
                      {m.name} {m.id === user.id ? "· you" : ""}
                    </Text>
                    <Text style={s.mutedSmall}>{Math.round(m.kcal)} kcal logged</Text>
                  </View>
                  {!m.share_diary && m.id !== user.id ? (
                    <Text>🔒</Text>
                  ) : (
                    <Text style={[s.pct, warn && { color: C.coral }]}>
                      {Math.round((m.kcal / Math.max(1, m.calorie_target)) * 100)}%
                    </Text>
                  )}
                </View>
                <View style={{ marginTop: 10 }}>
                  <Progress
                    value={(m.kcal / Math.max(1, m.calorie_target)) * 100}
                    tone={warn ? C.coral : C.green}
                  />
                </View>
                <Text style={[s.mutedSmall, { marginTop: 6 }]}>
                  {Math.round(m.kcal)} / {m.calorie_target} kcal
                </Text>
              </Pressable>
            );
          })}
          <Btn label="Leave family" onPress={leave} variant="outline" small />
        </>
      ) : (
        <>
          {[
            {
              icon: "key-outline",
              title: "Join a family",
              sub: "Got an invite code? Enter it to share diaries with your people.",
              input: (
                <TextInput
                  value={joinCode}
                  onChangeText={(v) => setJoinCode(v.toUpperCase())}
                  placeholder="e.g. DEMO42"
                  autoCapitalize="characters"
                  placeholderTextColor="#6f7f73"
                  style={s.input}
                />
              ),
              action: <Btn label="Join with code" onPress={join} disabled={busy} />,
            },
            {
              icon: "people-outline",
              title: "Start a family",
              sub: "Create a shared space, then invite people with a code.",
              input: (
                <TextInput
                  value={name}
                  onChangeText={setName}
                  placeholder="Family name (optional)"
                  placeholderTextColor="#6f7f73"
                  style={s.input}
                />
              ),
              action: <Btn label="Create family" onPress={create} variant="outline" disabled={busy} />,
            },
          ].map((c) => (
            <View key={c.title} style={[s.card, { gap: 10 }]}>
              <Ionicons name={c.icon as any} size={28} color={C.green} />
              <Text style={s.h3}>{c.title}</Text>
              <Text style={s.mutedSmall}>{c.sub}</Text>
              {c.input}
              {c.action}
            </View>
          ))}
        </>
      )}

      {member && (
        <MemberSheet token={token} member={member} onClose={() => setMember(null)} />
      )}
    </ScrollView>
  );
}

function MemberSheet({
  token,
  member,
  onClose,
}: {
  token: string;
  member: Member;
  onClose: () => void;
}) {
  const [day, setDay] = useState<MemberDay | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    request<MemberDay>(`/api/family/members/${member.id}/diary?date=${todayStr()}`, token)
      .then(setDay)
      .catch((e) => setError(msg(e)));
  }, [member.id, token]);

  return (
    <Modal visible animationType="slide" presentationStyle="pageSheet" onRequestClose={onClose}>
      <SafeAreaView style={{ flex: 1, backgroundColor: C.bg }}>
        <ScrollView contentContainerStyle={{ padding: 18, gap: 12 }}>
          <View style={s.rowBetween}>
            <View>
              <Text style={s.eyebrow}>Family diary</Text>
              <Text style={s.h2}>
                {member.name} · {prettyDate(todayStr())}
              </Text>
            </View>
            <Pressable onPress={onClose} style={s.closeBtn}>
              <Text style={{ fontSize: fs(22), color: C.muted }}>×</Text>
            </Pressable>
          </View>
          <Banner text={error} />
          {day ? (
            <>
              <View style={[s.card, { alignItems: "center" }]}>
                <Text style={[s.calBig, day.totals.kcal >= day.calorie_target * 0.8 && { color: C.coral }]}>
                  {Math.round(day.totals.kcal)}
                </Text>
                <Text style={s.mutedSmall}>of {day.calorie_target} kcal</Text>
                <View style={{ width: "100%", marginTop: 10 }}>
                  <Progress
                    value={(day.totals.kcal / Math.max(1, day.calorie_target)) * 100}
                    tone={day.totals.kcal >= day.calorie_target * 0.8 ? C.coral : C.green}
                  />
                </View>
              </View>
              <View style={s.card}>
                {day.entries.length === 0 ? (
                  <Text style={s.empty}>Nothing logged on this day.</Text>
                ) : (
                  day.entries.map((e) => (
                    <View key={e.id} style={s.entry}>
                      <View style={{ flex: 1 }}>
                        <Text style={s.entryName}>{e.name}</Text>
                        <Text style={s.mutedSmall}>
                          {e.meal} · {Math.round(e.grams)} g
                        </Text>
                      </View>
                      <Text style={s.entryKcal}>{Math.round(e.kcal)}</Text>
                    </View>
                  ))
                )}
              </View>
            </>
          ) : (
            <ActivityIndicator color={C.green} />
          )}
        </ScrollView>
      </SafeAreaView>
    </Modal>
  );
}

// ------------------------------------------------------------------ you

function You({
  token,
  user,
  onUser,
  onSignOut,
}: {
  token: string;
  user: User;
  onUser: (u: User) => void;
  onSignOut: () => void;
}) {
  const [share, setShare] = useState(user.share_diary);
  const [target, setTarget] = useState(String(user.calorie_target));
  const [savingTarget, setSavingTarget] = useState(false);
  const [error, setError] = useState("");

  const toggle = async () => {
    const next = !share;
    setShare(next);
    try {
      const u = await request<User>("/api/profile", token, {
        method: "PUT",
        body: JSON.stringify({ share_diary: next }),
      });
      onUser(u);
    } catch (e) {
      setError(msg(e));
      setShare(!next);
    }
  };

  const saveTarget = async () => {
    const n = Number(target);
    if (!Number.isFinite(n) || n < 800 || n > 6000) {
      setError("Calorie target must be between 800 and 6000.");
      return;
    }
    setSavingTarget(true);
    setError("");
    try {
      const u = await request<User>("/api/profile", token, {
        method: "PUT",
        body: JSON.stringify({ calorie_target: n }),
      });
      onUser(u);
    } catch (e) {
      setError(msg(e));
    }
    setSavingTarget(false);
  };

  return (
    <ScrollView contentContainerStyle={s.page}>
      <View style={{ flexDirection: "row", alignItems: "center", gap: 12 }}>
        <Avatar name={user.name} size={54} tone="green" />
        <View>
          <Text style={s.eyebrow}>Your settings</Text>
          <Text style={s.dateTitle}>Hi, {user.name}</Text>
        </View>
      </View>
      <Banner text={error} />

      <View style={s.card}>
        <Text style={s.cardTitle}>Daily calorie target</Text>
        <Text style={s.calBig}>{user.calorie_target}</Text>
        <Text style={s.mutedSmall}>
          Set your own target, or leave the auto-calculated one from onboarding.
        </Text>
        <View style={{ flexDirection: "row", alignItems: "flex-end", gap: 8, marginTop: 10 }}>
          <View style={{ flex: 1 }}>
            <Text style={s.fieldLabel}>Calories / day</Text>
            <TextInput
              value={target}
              onChangeText={setTarget}
              keyboardType="numeric"
              style={s.input}
            />
          </View>
          <Btn
            label={savingTarget ? "Saving…" : "Save"}
            onPress={saveTarget}
            disabled={savingTarget}
            small
          />
        </View>
      </View>

      <Pressable onPress={toggle} style={[s.card, { flexDirection: "row", alignItems: "center", gap: 12 }]}>
        <View style={{ flex: 1 }}>
          <Text style={s.cardTitle}>Share my diary</Text>
          <Text style={s.mutedSmall}>Let your family see your meals and calories.</Text>
        </View>
        <View style={[s.toggle, share && { backgroundColor: C.greenDeep }]}>
          <View style={[s.knob, share && { alignSelf: "flex-end" }]} />
        </View>
      </Pressable>

      <Btn label="Sign out" onPress={onSignOut} variant="outline" />
      <Text style={[s.tiny, { textAlign: "center" }]}>
        Scan photos and descriptions stay in your scan history. Session on this device only.
      </Text>
    </ScrollView>
  );
}

// ------------------------------------------------------------------ styles

const s = StyleSheet.create({
  center: { flex: 1, alignItems: "center", justifyContent: "center", backgroundColor: C.bg },
  markLarge: {
    width: 64,
    height: 64,
    borderRadius: 20,
    backgroundColor: C.green,
    alignItems: "center",
    justifyContent: "center",
  },
  markLargeText: { fontFamily: "Georgia", fontSize: fs(42), color: C.lime, marginTop: -6 },
  mark: {
    width: 40,
    height: 40,
    borderRadius: 13,
    backgroundColor: C.greenDeep,
    alignItems: "center",
    justifyContent: "center",
  },
  markText: { fontFamily: "Georgia", fontSize: fs(28), color: C.lime, marginTop: -4 },
  brandRow: { flexDirection: "row", alignItems: "center", gap: 10 },
  brand: { fontFamily: "Georgia", fontSize: fs(23), color: C.ink },
  login: { padding: 26, gap: 16, flexGrow: 1, justifyContent: "center" },
  h1: {
    fontFamily: "Georgia",
    fontSize: fs(44),
    lineHeight: 48,
    letterSpacing: -1.2,
    color: C.ink,
    marginTop: 10,
  },
  h2: { fontFamily: "Georgia", fontSize: fs(27), letterSpacing: -0.5, color: C.ink },
  h3: { fontFamily: "Georgia", fontSize: fs(23), color: C.ink },
  lede: { color: C.muted, fontSize: fs(15), lineHeight: 22 },
  eyebrow: {
    color: C.green,
    fontSize: fs(10.5),
    fontWeight: "800",
    letterSpacing: 1.4,
    textTransform: "uppercase",
  },
  chips: { flexDirection: "row", flexWrap: "wrap", gap: 7 },
  chip: { backgroundColor: C.card, borderWidth: 1, borderColor: C.line, borderRadius: 999, paddingHorizontal: 11, paddingVertical: 6 },
  chipText: { fontSize: fs(11.5), fontWeight: "600", color: C.muted },
  btn: { borderRadius: 999, paddingVertical: 13, paddingHorizontal: 20, alignItems: "center", justifyContent: "center" },
  btnSmall: { paddingVertical: 9, paddingHorizontal: 14 },
  btnPrimary: { backgroundColor: C.greenDeep },
  btnGhost: { backgroundColor: C.soft },
  btnOutline: { backgroundColor: C.card, borderWidth: 1, borderColor: C.line },
  btnLime: { backgroundColor: C.lime },
  btnText: { fontSize: fs(14), fontWeight: "700", color: C.ink },
  btnTextLight: { color: "#fff" },
  input: {
    borderWidth: 1,
    borderColor: C.line,
    backgroundColor: C.card,
    borderRadius: 12,
    paddingHorizontal: 13,
    paddingVertical: 11,
    fontSize: fs(15),
    color: C.ink,
  },
  fieldLabel: { fontSize: fs(12), fontWeight: "700", color: C.muted, marginBottom: 2 },
  banner: {
    backgroundColor: "#3a1d15",
    borderWidth: 1,
    borderColor: "#5a2f21",
    borderRadius: 12,
    padding: 11,
  },
  onboard: { padding: 26, gap: 12, flexGrow: 1, justifyContent: "center" },
  tiny: { fontSize: fs(11.5), color: C.muted },
  page: { padding: 18, paddingBottom: 36, gap: 14 },
  topRow: { flexDirection: "row", justifyContent: "space-between", alignItems: "flex-start" },
  dateTitle: { fontFamily: "Georgia", fontSize: fs(36), letterSpacing: -1, color: C.ink },
  dateNav: { flexDirection: "row", alignItems: "center", gap: 4 },
  chev: { width: 28, height: 28, borderRadius: 999, borderWidth: 1, borderColor: C.line, alignItems: "center", justifyContent: "center", backgroundColor: C.card },
  chevText: { fontSize: fs(16), color: C.muted, marginTop: -2 },
  todayBtn: { paddingHorizontal: 10, paddingVertical: 6, borderRadius: 999, backgroundColor: C.card, borderWidth: 1, borderColor: C.line },
  todayBtnText: { fontSize: fs(11), fontWeight: "700", color: C.ink },
  card: { backgroundColor: C.card, borderWidth: 1, borderColor: C.line, borderRadius: 20, padding: 16 },
  cardTitle: { fontSize: fs(14), fontWeight: "800", color: C.ink },
  rowBetween: { flexDirection: "row", alignItems: "center", justifyContent: "space-between", gap: 10 },
  mutedSmall: { fontSize: fs(11.5), color: C.muted },
  calRow: { flexDirection: "row", alignItems: "flex-end", gap: 12, marginVertical: 6 },
  calBig: { fontFamily: "Georgia", fontSize: fs(50), lineHeight: 54, letterSpacing: -1.5, color: C.ink },
  remaining: { fontSize: fs(12), fontWeight: "700", color: C.green },
  track: { height: 8, borderRadius: 99, backgroundColor: "#24322a", overflow: "hidden", marginTop: 6 },
  fill: { height: "100%", borderRadius: 99 },
  scanCta: {
    flexDirection: "row",
    alignItems: "center",
    gap: 9,
    backgroundColor: "#1a2620",
    borderRadius: 14,
    padding: 12,
    marginTop: 14,
  },
  scanCtaText: { fontSize: fs(13), fontWeight: "700", color: C.ink },
  mealHead: { flexDirection: "row", alignItems: "center", justifyContent: "space-between", marginBottom: 6 },
  mealTitle: { fontSize: fs(15), fontWeight: "800", color: C.ink },
  addBtn: { width: 27, height: 27, borderRadius: 999, backgroundColor: C.soft, alignItems: "center", justifyContent: "center" },
  addBtnText: { fontSize: fs(18), color: C.green, marginTop: -2 },
  empty: { fontSize: fs(12), color: C.muted, paddingVertical: 10 },
  entry: { flexDirection: "row", alignItems: "center", gap: 10, paddingVertical: 10, borderTopWidth: 1, borderTopColor: C.line },
  entryName: { fontSize: fs(13.5), fontWeight: "600", color: C.ink },
  entryKcal: { fontSize: fs(13), fontWeight: "700", color: C.ink },
  tabbar: {
    flexDirection: "row",
    backgroundColor: "#0f1813",
    borderTopWidth: 1,
    borderTopColor: C.line,
    paddingVertical: 7,
    paddingHorizontal: 8,
  },
  tab: { flex: 1, alignItems: "center", gap: 2, paddingVertical: 6, borderRadius: 12 },
  tabActive: { backgroundColor: C.soft },
  tabText: { fontSize: fs(10), color: C.muted, fontWeight: "600" },
  selName: { fontFamily: "Georgia", fontSize: fs(22), color: C.ink },
  previewKcal: { fontFamily: "Georgia", fontSize: fs(24), color: C.green },
  foodRow: {
    flexDirection: "row",
    alignItems: "center",
    gap: 10,
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: C.line,
  },
  foodName: { fontSize: fs(14), fontWeight: "600", color: C.ink },
  foodKcal: { fontSize: fs(12.5), fontWeight: "700", color: C.muted },
  closeBtn: { width: 34, height: 34, borderRadius: 999, backgroundColor: C.card, borderWidth: 1, borderColor: C.line, alignItems: "center", justifyContent: "center" },
  preview: { width: "100%", height: 240, borderRadius: 16 },
  scanDrop: { alignItems: "center", justifyContent: "center", gap: 6, paddingVertical: 36, borderWidth: 1.5, borderStyle: "dashed", borderColor: C.line, borderRadius: 18, backgroundColor: "#101a14" },
  invite: { fontFamily: Platform.select({ ios: "Menlo", default: "monospace" }), fontSize: fs(30), fontWeight: "700", letterSpacing: 3, color: C.green },
  memberName: { fontSize: fs(14), fontWeight: "700", color: C.ink },
  pct: { fontSize: fs(13), fontWeight: "800", color: C.green },
  wrap: { flexDirection: "row", flexWrap: "wrap", gap: 7, marginTop: 4 },
  row: { flexDirection: "row", gap: 6 },
  pill: { borderWidth: 1, borderColor: C.line, backgroundColor: C.card, borderRadius: 999, paddingHorizontal: 13, paddingVertical: 7 },
  pillActive: { backgroundColor: C.green, borderColor: C.green },
  pillText: { fontSize: fs(12), fontWeight: "600", color: C.muted },
  pillTextActive: { color: "#fff" },
  toggle: { width: 46, height: 27, borderRadius: 99, backgroundColor: "#3a4a40", padding: 3, justifyContent: "center" },
  knob: { width: 21, height: 21, borderRadius: 99, backgroundColor: "#fff" },
});

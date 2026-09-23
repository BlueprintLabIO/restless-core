const $ = (id) => document.getElementById(id);
const params = new URLSearchParams(location.search);
let invitation = params.get("invite");
let mode = params.has("token") ? "reset" : "sign-in";
function message(id, text) {
  $(id).textContent = text;
  $(id).hidden = !text;
}
function clear() {
  message("error", "");
  message("status", "");
}
async function api(path, body) {
  const response = await fetch(path, {
    method: body === undefined ? "GET" : "POST",
    headers: body === undefined ? {} : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const result = await response.json();
  if (!response.ok) {
    const error = Error(result.message || "Please try again.");
    error.status = response.status;
    throw error;
  }
  return result;
}
async function action(button, work) {
  clear();
  button.disabled = true;
  try {
    await work();
  } catch (error) {
    message("error", error.message);
  } finally {
    button.disabled = false;
  }
}
function setMode(next) {
  mode = next;
  clear();
  const signup = mode === "sign-up",
    forgot = mode === "forgot",
    reset = mode === "reset";
  $("form-heading").textContent = signup
    ? "Create your account"
    : forgot
      ? "Reset your password"
      : reset
        ? "Choose a new password"
        : "Sign in";
  $("submit-account").textContent = signup
    ? "Create account"
    : forgot
      ? "Send reset email"
      : reset
        ? "Save password"
        : "Sign in";
  $("name-field").hidden = !signup;
  $("name").required = signup;
  $("password-field").hidden = forgot;
  $("password").required = !forgot;
  $("password").minLength = signup || reset ? 12 : 1;
  $("password").title = signup || reset ? "Use at least 12 characters." : "";
  $("password").autocomplete =
    signup || reset ? "new-password" : "current-password";
  $("email").closest("label").hidden = reset;
  $("email").required = !reset;
  $("switch-mode").textContent =
    mode === "sign-in" ? "Create an account" : "Back to sign in";
  $("forgot-password").hidden = mode !== "sign-in";
  $("resend-verification").hidden = true;
}
async function refresh() {
  if (mode === "reset") {
    $("authentication").hidden = false;
    $("company").hidden = true;
    $("sign-out").hidden = true;
    return;
  }
  let state;
  try {
    state = await api("/api/company");
  } catch (error) {
    if (error.status !== 401) throw error;
    state = null;
  }
  $("authentication").hidden = !!state;
  $("company").hidden = !state;
  $("sign-out").hidden = !state;
  $("accept-invitation").hidden = !state || !invitation;
  if (!state) return;
  $("account-identity").textContent =
    `${state.user.name} · ${state.user.email}`;
  $("enter-company").hidden = !state.role;
  $("bootstrap-company").hidden = !state.canBootstrap;
  $("no-access").hidden = !!state.role || state.canBootstrap || !!invitation;
  $("suspended").hidden = state.status !== "suspended";
  if (state.status === "suspended") $("no-access").hidden = true;
}
$("switch-mode").addEventListener("click", () =>
  setMode(mode === "sign-in" ? "sign-up" : "sign-in"),
);
$("forgot-password").addEventListener("click", () => setMode("forgot"));
$("account-form").addEventListener("submit", (event) => {
  event.preventDefault();
  void action($("submit-account"), async () => {
    const email = $("email").value.trim(),
      password = $("password").value;
    if (mode === "sign-up") {
      await api("/api/auth/sign-up/email", {
        name: $("name").value.trim(),
        email,
        password,
        callbackURL: location.href,
      });
      setMode("sign-in");
      $("password").value = "";
      message(
        "status",
        "Check your email to verify your account, then sign in.",
      );
      $("resend-verification").hidden = false;
    } else if (mode === "forgot") {
      await api("/api/auth/request-password-reset", {
        email,
        redirectTo: location.origin + "/",
      });
      message(
        "status",
        "If an account exists for this email, a reset link is on its way.",
      );
    } else if (mode === "reset") {
      await api("/api/auth/reset-password", {
        newPassword: password,
        token: params.get("token"),
      });
      history.replaceState(null, "", "/");
      setMode("sign-in");
      $("password").value = "";
      message("status", "Password updated. Sign in to continue.");
    } else {
      try {
        await api("/api/auth/sign-in/email", { email, password });
        $("password").value = "";
        await refresh();
      } catch (error) {
        $("resend-verification").hidden = false;
        throw error;
      }
    }
  });
});
$("resend-verification").addEventListener("click", () =>
  action($("resend-verification"), async () => {
    if (!$("email").validity.valid)
      throw Error("Enter your email address first.");
    await api("/api/auth/send-verification-email", {
      email: $("email").value.trim(),
      callbackURL: location.href,
    });
    message("status", "Verification email requested. Check your inbox.");
  }),
);
$("sign-out").addEventListener("click", () =>
  action($("sign-out"), async () => {
    await api("/api/auth/sign-out", {});
    await refresh();
    setMode("sign-in");
  }),
);
$("bootstrap-company").addEventListener("click", () =>
  action($("bootstrap-company"), async () => {
    await api("/api/company/bootstrap", {});
    await refresh();
  }),
);
$("accept-invitation").addEventListener("click", () =>
  action($("accept-invitation"), async () => {
    await api("/api/invitations/accept", { id: invitation });
    $("invitation").hidden = true;
    invitation = null;
    history.replaceState(null, "", "/");
    await refresh();
    message("status", "You’ve joined the company.");
  }),
);
$("enter-company").addEventListener("click", () =>
  action($("enter-company"), async () => {
    const entry = await api("/api/enter", {});
    const form = document.createElement("form");
    form.method = "POST";
    form.action = entry.action;
    const token = document.createElement("input");
    token.type = "hidden";
    token.name = "assertion";
    token.value = entry.assertion;
    form.append(token);
    document.body.append(form);
    form.submit();
  }),
);
try {
  const settings = await api("/api/settings");
  document.title = `${settings.companyName} · Restless`;
  document
    .querySelectorAll(".company-name")
    .forEach((element) => (element.textContent = settings.companyName));
  $("invitation").hidden = !invitation;
  setMode(mode);
  if (params.has("error"))
    message(
      "error",
      "That account link could not be used. Request a new link or sign in again.",
    );
  await refresh();
} catch {
  message("error", "The account service is unavailable. Please try again.");
}

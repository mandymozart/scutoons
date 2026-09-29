const STORAGE_KEY = 'scutoons_gen_token';

export const getToken   = () => localStorage.getItem(STORAGE_KEY) || '';
export const clearToken = () => localStorage.removeItem(STORAGE_KEY);
const setToken          = (t) => localStorage.setItem(STORAGE_KEY, t);

export async function validateToken(token) {
    try {
        const r = await fetch('auth.php', {
            method:  'POST',
            headers: { 'Content-Type': 'application/json' },
            body:    JSON.stringify({ action: 'validate', token })
        });
        return (await r.json()).valid === true;
    } catch {
        return false;
    }
}

export async function loginWithPassword(password) {
    const r = await fetch('auth.php', {
        method:  'POST',
        headers: { 'Content-Type': 'application/json' },
        body:    JSON.stringify({ action: 'login', password })
    });
    const d = await r.json();
    if (d.token) { setToken(d.token); return d.token; }
    throw new Error(d.error || 'Invalid password');
}

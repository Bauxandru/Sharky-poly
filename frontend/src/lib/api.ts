export const API_URL =
  process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080";

export const fetcher = (url: string) =>
  fetch(url).then((res) => {
    if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
    return res.json();
  });

/** Subscribe to real-time orderbook updates via WebSocket */
export function subscribeToOrderbook(
  orderbook: string,
  onUpdate: (msg: any) => void
): () => void {
  const wsUrl = (API_URL.replace("http", "ws") + "/ws");
  const ws = new WebSocket(wsUrl);

  ws.onopen = () => {
    // Subscribe to specific orderbook events
    ws.send(JSON.stringify({ subscribe: orderbook }));
  };

  ws.onmessage = (e) => {
    try {
      const msg = JSON.parse(e.data);
      if (msg.orderbook === orderbook || !msg.orderbook) {
        onUpdate(msg);
      }
    } catch {}
  };

  ws.onerror = (e) => console.warn("WS error", e);

  return () => ws.close();
}

// The SDK is served by Nebo at /sdk/nebo.global.js (loaded in index.html)
// and defines one global, NeboAppSDK. There is no bare `nebo` global.
const { nebo } = window.NeboAppSDK;

const STAGES = ['prospect', 'analysis', 'negotiation', 'closed'];
let deals = [];

// Mount embedded chat
nebo.chat.mount(document.getElementById('chat-container'), {
  placeholder: 'Ask about deals, analyze documents...',
  theme: 'dark',
});

// Deals live in the app's own storage: no sidecar needed.
async function loadDeals() {
  try {
    deals = (await nebo.storage.getItem('deals')) || [];
    renderPipeline(deals);
  } catch (err) {
    console.error('Failed to load deals:', err);
  }
}

async function saveDeals() {
  await nebo.storage.setItem('deals', deals);
  nebo.chat.setContext({ pipeline: deals });
}

function renderPipeline(deals) {
  for (const stage of STAGES) {
    const list = document.querySelector(`[data-stage="${stage}"] .deal-list`);
    list.innerHTML = '';
    const stageDeals = deals.filter(d => d.stage === stage);
    for (const deal of stageDeals) {
      const card = document.createElement('div');
      card.className = 'deal-card';
      const title = document.createElement('h3');
      title.textContent = deal.name;
      const amount = document.createElement('div');
      amount.className = 'amount';
      amount.textContent = `$${deal.amount.toLocaleString()}`;
      card.append(title, amount);
      card.addEventListener('click', () => openDeal(deal.id));
      list.appendChild(card);
    }
  }
}

function openDeal(id) {
  const deal = deals.find(d => d.id === id);
  nebo.chat.setContext({ deal, pipeline: deals });
  nebo.chat.send(`Show me details for deal ${deal ? deal.name : id}`);
}

document.getElementById('new-deal-btn').addEventListener('click', async () => {
  const name = window.prompt('Deal name');
  if (!name) return;
  const amount = Number(window.prompt('Deal amount', '0')) || 0;
  deals.push({ id: crypto.randomUUID(), name, amount, stage: 'prospect', created_at: new Date().toISOString() });
  await saveDeals();
  renderPipeline(deals);
});

// Initial load
loadDeals();

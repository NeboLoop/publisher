import { nebo } from '@neboai/app-sdk';

// Mount embedded chat
nebo.chat.mount(document.getElementById('chat-container'), {
  placeholder: 'Ask about deals, analyze documents...',
  theme: 'dark',
});

// Load deals from sidecar
async function loadDeals() {
  try {
    const resp = await nebo.fetch('/deals');
    if (resp.ok) {
      const deals = await resp.json();
      renderPipeline(deals);
    }
  } catch (err) {
    console.error('Failed to load deals:', err);
  }
}

function renderPipeline(deals) {
  const stages = ['prospect', 'analysis', 'negotiation', 'closed'];
  for (const stage of stages) {
    const list = document.querySelector(`[data-stage="${stage}"] .deal-list`);
    list.innerHTML = '';
    const stageDeals = deals.filter(d => d.stage === stage);
    for (const deal of stageDeals) {
      const card = document.createElement('div');
      card.className = 'deal-card';
      card.innerHTML = `
        <h3>${deal.name}</h3>
        <div class="amount">$${deal.amount.toLocaleString()}</div>
      `;
      card.addEventListener('click', () => openDeal(deal.id));
      list.appendChild(card);
    }
  }
}

function openDeal(id) {
  nebo.chat.send(`Show me details for deal ${id}`);
}

document.getElementById('new-deal-btn').addEventListener('click', () => {
  nebo.chat.send('Create a new deal');
});

// Initial load
loadDeals();

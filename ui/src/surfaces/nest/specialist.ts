import type { AgentProfile, Bridge, ChatDraft } from "../../bridge";
import type { ChatBackend, ChatPanelOptions } from "../../island/chat";
import { t } from "../../i18n";

export interface SpecialistChatBackend {
  current: typeof Bridge.specialistChatCurrent;
  choice?: typeof Bridge.specialistChatChoiceSet;
  pickFiles?: typeof Bridge.specialistChatPickFiles;
  conversations: typeof Bridge.specialistChatConversations;
  send: typeof Bridge.specialistChatSend;
  open: typeof Bridge.specialistChatOpen;
  reset: typeof Bridge.specialistChatReset;
  stop: typeof Bridge.specialistChatStop;
  decide: typeof Bridge.specialistChatDecide;
  saveDraft: typeof Bridge.specialistChatDraftSet;
  rename: typeof Bridge.specialistChatRename;
  remove: typeof Bridge.specialistChatDelete;
}

/** The production ChatPanel keeps one owner for messages, drafts, history and approvals. */
export function specialistBackend(api: SpecialistChatBackend, profile: AgentProfile, identity: NonNullable<ChatPanelOptions["specialist"]>): ChatBackend {
  let reading: ReturnType<SpecialistChatBackend["current"]> | null = null;
  const identify = (snapshot: Awaited<ReturnType<SpecialistChatBackend["current"]>>) => {
    identity.choice = snapshot.choice ?? null;
    identity.name = snapshot.profileTitle;
    identity.revision = snapshot.profileRevision;
    identity.species = snapshot.profileSpecies;
    identity.look = snapshot.profileLook;
    return snapshot;
  };
  const read = () => reading ??= api.current(profile.id).then(identify).finally(() => { reading = null; });
  const generation = (value?: number) => {
    if (value === undefined) throw new Error(t("The conversation changed before sending. Review your draft and try again."));
    return value;
  };
  return {
    chooseFiles: api.pickFiles ? (title,expected) => api.pickFiles!(profile.id,generation(expected),title) : undefined,
    setSpecialistChoice: api.choice ? async (choice,expected) => { return api.choice!(profile.id,choice,generation(expected)); } : undefined,
    current: () => read().then(snapshot => snapshot.conversation),
    draft: () => read().then(snapshot => ({ ...snapshot.draft, generation: snapshot.generation })),
    saveDraft: (draft: ChatDraft, expected) => api.saveDraft(profile.id, draft, generation(expected)),
    send: (text, files, folder, mentions, expected) => {
      if (mentions?.length) return Promise.reject("Specialist conversations do not accept voice commands or flock mentions.");
      return api.send(profile.id, text, generation(expected), folder, files);
    },
    decide: (id, answer) => api.decide(profile.id, id, answer),
    stop: () => api.stop(profile.id),
    reset: (_provider, expected) => api.reset(profile.id, generation(expected)),
    models: async () => [], pick: async () => ({}), setPick: async () => {},
    conversations: () => api.conversations(profile.id),
    open: (id, expected) => api.open(profile.id, id, generation(expected)),
    rename: (id, title) => api.rename(profile.id, id, title),
    remove: async id => {
      const result = await api.remove(profile.id, id);
      if (result.reset) return { ...result.reset, current: result.current };
      return result.current;
    },
    keyboard: () => {},
  };
}

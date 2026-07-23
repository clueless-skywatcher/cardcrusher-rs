YoureInDanger = Card:new(10312660)

local activate = YoureInDanger:add_effect()
activate.frequency = {HARD_PER_TURN, 1}

function activate:condition(effect)
    return effect:free_monster_zones(YOU) >= 1
        and effect:deck(YOU):with_archetype(DANGER):distinct_names().count >= 3
end

function activate:resolve(effect)
    -- Reveal 3 differently-named Danger! monsters from your Deck
    local three = effect:prompt_selection(effect:deck(YOU):with_archetype(DANGER):distinct_names(), 3)
    effect:reveal(three)
    -- Opponent randomly picks 1 of them to Special Summon to your field
    local chosen = effect:random_pick(three, OPPONENT, 1)
    effect:perform_special_summon(chosen, YOU)
    -- That monster is destroyed during the End Phase
    effect:queue(END_PHASE_BEGAN, function() effect:destroy(chosen) end)
end

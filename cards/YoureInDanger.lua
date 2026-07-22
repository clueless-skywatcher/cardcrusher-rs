YoureInDanger = Card:new(10312660)

local activate = YoureInDanger:add_effect()
activate.frequency = {HARD_PER_TURN, 1}

function activate:condition(e)
    return e:free_monster_zones(YOU) >= 1
        and e:deck(YOU):with_archetype(DANGER):distinct_names().count >= 3
end

function activate:resolve(e)
    -- Reveal 3 differently-named Danger! monsters from your Deck
    local three = e:prompt_selection(e:deck(YOU):with_archetype(DANGER):distinct_names(), 3)
    e:reveal(three)
    -- Opponent randomly picks 1 of them to Special Summon to your field
    local chosen = e:random_pick(three, OPPONENT, 1)
    e:perform_special_summon(chosen, YOU)
    -- That monster is destroyed during the End Phase
    e:queue(END_PHASE_BEGAN, function() e:destroy(chosen) end)
end

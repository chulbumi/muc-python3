# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def setTanker(self, ob, target):
        ob.act = ACT_FIGHT
        target.act = ACT_FIGHT
        if target in ob.target:
            ob.target.remove(target)
        ob.target.insert(0, target)

    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법 : [무리원이름] 방어지정')
        if ob.Party == None:
            ob.sendLine('☞ 당신이 속한 무리가 없어요. ^^')
            return
        if ob.env.checkAttr('전투금지'):
            ob.sendLine('☞ 이곳에선 모든 전투가 금지되어 있어요. ^^')
            return
        if line.find('시체') != -1:
            ob.sendLine('☞ 강호에는 공격할 수 있는것과 없는것이 있지!')
            return
        group = ob.Party
        if line == group['이름']:
            tanker=ob
        else:
            tanker=ob.env.findObjName(line)
        ply = None
        if ob != group:
            ob.sendLine('☞ 당신은 무리의 대장이 아니에요. ^^')
            return
        if ob!=tanker and tanker not in group.PartyMember:
            ob.sendLine('☞ 당신의 무리원이 아니에요.')
            return
        if tanker == None or is_player(tanker) == False:
            ob.sendLine('☞ 지정할 대상을 찾지 못했어요.')
            return

        if group.act ==ACT_FIGHT:
            ply = group
        else:
            for member in group.PartyMember:
                if member.act!= ACT_FIGHT: 
                    continue
                else:
                    ply = member
                    break

        if ply == None:
            ob.sendLine('☞ 전투중인 무리원이 없어요. ^^')
            return            
        target = copy.copy(ply.target)      
        tanker.target = copy.copy(ply.target)      
        for mob in target:
            self.setTanker(mob, tanker)
            if is_player(mob):
                mob.fightMode = True
                
        ob.sendToParty('%s 무리의 방어로 지정 되었습니다.' % tanker.han_iga(), ex = tanker, prompt = True)
        tanker.sendLine('\r\n당신이 무리의 방어로 지정 되었습니다.')

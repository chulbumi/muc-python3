# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def setAllTarget(self, ob, target):
        ob.act = ACT_FIGHT
        target.act = ACT_FIGHT
        if target not in ob.target:
            ob.target.append(target)
        #else:
            #ob.target.remove(target)
            #ob.target.append(target)

    def cmd(self, ob, line):
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
        ply = None
        if line == '':
            if ob != group:
                ob.sendLine('☞ 당신은 무리의 대장이 아니에요. ^^')
                return
            elif ob.act== ACT_FIGHT:
                ply=ob            
            else:
                for member in group.PartyMember:
                    if member.act!= ACT_FIGHT: 
                        continue
                    else:
                        ply = member
                        break
        else:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return

        if ply == None:
            ob.sendLine('☞ 무리 합동공격할 대상이 없어요. ^^')
            return            
        target = copy.copy(ply.target)      
        if ob!=ACT_FIGHT:
            for mob in target:
                self.setAllTarget(ob, mob)
                self.setAllTarget(mob, ob)
        for member in group.PartyMember:
            if member.act== ACT_FIGHT: 
                continue
            for mob in target:
                self.setAllTarget(member, mob)
                self.setAllTarget(mob, member)
                if is_player(mob):
                    mob.fightMode = True
        ob.sendToParty('당신이 속한 무리가 무리합동 공격을 시작 합니다.', prompt = True)


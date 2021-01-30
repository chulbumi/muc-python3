# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
    level = 1000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
        if len(line) == 0:
            ob.sendLine('사용법: [사용자 이름] 사용자몹제거')
            return

        for ply in ob.channel.players:
            if ply['이름'] == line:
#mob = ob.env.findObjName(line)
#        if mob == None or is_player(mob) == False:
#            ob.sendLine('그런 몹이 없어요!')
#            return
                ply.env.objs.remove(ply)
                ob.channel.players.remove(ply)
                del ply
                ob.sendLine('사용자몹이 제거되었습니다.')

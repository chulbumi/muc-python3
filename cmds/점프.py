# -*- coding: utf-8 -*-
from objs.cmd import Command

class CmdObj(Command):

    def down(self, ob):
        ob.set('cooltime', 0)
        ob.sendLine('당신은 안전하게 착지합니다. ^^v')

    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^*')
            return
            
        if ob.get('cooltime') == None:
            ob.set('cooltime', 0)
        if ob.get('cooltime') == 1:
            ob.sendLine('기술을 쓰기엔 너무도 바빠요~')
            return
        from twisted.internet import reactor
        ob.sendLine('당신이 부웅~~ 날아 오릅니다')
        ob.set('cooltime', 1)
        reactor.callLater(2, self.down, ob)

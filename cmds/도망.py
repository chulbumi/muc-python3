# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cool(self, ob):
        ob['_runaway'] = 0

    def cmd(self, ob, line):
        if ob.act != ACT_FIGHT:
            ob.sendLine('☞ 무림인은 아무때나 도망가는것이 아니라네')
            return
        if ob.env.checkAttr('도망금지'):
            ob.sendLine('☞ 도망 갈려다 잡혔어요. \'흑흑~~ T_T\'')
            return
        if ob['_runaway'] == None:
            ob['_runaway'] = 0
        if ob['_runaway'] == 1:
            ob.sendLine('☞ 도망 갈려다 잡혔어요. \'흑흑~~ T_T\'')
            return
        ob['_runaway'] = 1
        from twisted.internet import reactor
        reactor.callLater(1, self.cool, ob)
        bonus = 0

        try:
            if ob.cooltime['능파미보'] == 1:
                bonus = 40
        except:
            pass

        mob = ob.target[0]
        c1 = mob['레벨'] * (mob.getDex() + 1) - ob['레벨'] * (ob.getDex() + 1)
        if c1 < 1:
            c1 = 1
        c1 = 100 - c1
        if c1 < 10:
            c1 = 10
        c1 += bonus
        c2 = randint(0, 100)
        if c2 > c1:
            ob.sendLine('☞ 도망 갈려다 잡혔어요. \'흑흑~~ T_T\'')
            return
        
        room, dir = ob.env.getRandomExit()
        if room == None or room == ob.env:
            ob.sendLine('☞ 도망 갈려다 잡혔어요. \'흑흑~~ T_T\'')
            return
            
        if getInt(room['레벨제한']) > ob['레벨']:
            ob.sendLine('☞ 도망 갈려다 잡혔어요. \'흑흑~~ T_T\'')
            return
        if room.checkLimitNum():
            ob.sendLine('☞ 도망 갈려다 잡혔어요. \'흑흑~~ T_T\'')
            return
        if room.checkAttr('사파출입금지') and ob['성격'] == '사파':
            ob.sendLine('☞ 도망 갈려다 잡혔어요. \'흑흑~~ T_T\'')
            return
        if room.checkAttr('정파출입금지') and ob['성격'] == '정파':
            ob.sendLine('☞ 도망 갈려다 잡혔어요. \'흑흑~~ T_T\'')
            return
        
        if ob.enterRoom(room, dir, '도망') == False:
            ob.sendLine('☞ 도망 갈려다 잡혔어요. \'흑흑~~ T_T\'')
            return


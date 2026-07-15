# -*- coding: utf-8 -*-

from objs.cmd import Command
from lib.hangul import *


class CmdObj(Command):

    def cmd(self, ob, line):
        words = line.split()
        if len(words) < 2:
            ob.sendLine('☞ 사용법: [대상] [암기] 투척')
            return
        if ob.env.checkAttr('전투금지'):
            ob.sendLine('☞ 이곳에선 모든 전투가 금지되어 있어요. ^^')
            return

        targetName = words[0]
        itemName = ' '.join(words[1:])
        mob = ob.env.findObjName(targetName)
        if mob == None or is_mob(mob) == False or mob['몹종류'] != 1 or mob.act > ACT_FIGHT:
            ob.sendLine('☞ 그 대상에게 암기를 날릴 수 없어요.')
            return

        item = ob.findObjInven(itemName)
        if item == None or item['종류'] != '암기':
            ob.sendLine('☞ 던질 만한 암기가 소지품에 없어요.')
            return

        wasFighting = ob.act == ACT_FIGHT
        if wasFighting and mob not in ob.target:
            ob.sendLine('☞ 현재의 비무에 신경을 집중하세요. @_@')
            return
        if wasFighting and ob.getTemp('암기투척틱') == ob.tick:
            ob.sendLine('☞ 아직 다음 암기를 꺼낼 틈이 나지 않았어요.')
            return
        if wasFighting == False and len(ob.target) != 0:
            ob.sendLine('☞ 현재의 비무에 신경을 집중하세요. @_@')
            return

        damage = getInt(item['교전위력'] if wasFighting else item['급습위력'])
        if damage < 1:
            damage = 1
        itemA = '\x1b[1;36m%s\x1b[0;37m' % item['이름']
        targetA = mob.getNameA()
        if wasFighting:
            own = '당신이 난전 속에서 %s을 날리지만, %s의 경계에 막혀 위력이 크게 줄어듭니다. (\x1b[1;31m- %d\x1b[0;37m)' % (itemA, targetA, damage)
            room = '%s 난전 속에서 %s을 날리지만, %s의 경계에 막혀 위력이 크게 줄어듭니다.' % (ob.han_iga(), itemA, targetA)
        else:
            own = '당신이 소매를 떨치자 %s 한 점이 섬광처럼 날아가 %s의 허점을 꿰뚫습니다. (\x1b[1;31m- %d\x1b[0;37m)' % (itemA, targetA, damage)
            room = '%s 소매를 떨치자 %s 한 점이 섬광처럼 날아가 %s의 허점을 꿰뚫습니다.' % (ob.han_iga(), itemA, targetA)

        if wasFighting == False:
            ob.setFight(mob)
        ob.remove(item)
        ob.setTemp('암기투척틱', ob.tick)
        ob.sendLine(own)
        ob.sendFightScriptRoom(room)
        mob.minusHP(damage, who = ob['이름'])

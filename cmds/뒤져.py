# -*- coding: utf-8 -*-

from objs.cmd import Command
from lib.hangul import *

class CmdObj(Command):

    def cmd(self, ob, line):
        if len(line) == 0:
            ob.sendLine('☞ 사용법: [대상] 뒤져')
            return
        obj = ob.env.findObjName(line)

        if obj == None or is_item(obj) or is_box(obj):
            ob.sendLine('☞ 뒤질대상이 없어요. ^^')
            return
        if is_player(obj):
            ob.sendLine('당신이 %s의 몸을 더듬습니다. "뭐 없나~~ -.-"' % obj.getNameA())
            ob.sendRoom('%s %s의 몸을 더듬습니다. "뭐 없나~~ -.-"' % (ob.han_iga(), obj.getNameA()))
            return
            
        if obj.act != ACT_DEATH and obj['몹종류'] != 6:
            ob.sendLine('당신이 %s의 몸을 더듬습니다. "뭐 없나~~ -.-"' % obj.getNameA())
            ob.sendRoom('%s %s의 몸을 더듬습니다. "뭐 없나~~ -.-"' % (ob.han_iga(), obj.getNameA()))
            return
        
        if len(obj.objs) == 0:
            if obj.act == ACT_DEATH:
                ob.sendLine('당신이 %s의 시체를 뒤집니다. \'뒤적~ 뒤적~\'' % obj.getNameA())
                ob.sendRoom('%s %s의 시체를 뒤집니다. \'뒤적~ 뒤적~\'' % (ob.han_iga(), obj.getNameA()))
            else:
                ob.sendLine('당신이 %s 뒤집니다. \'뒤적~ 뒤적~\'' % obj.han_obj())
                ob.sendRoom('%s %s 뒤집니다. \'뒤적~ 뒤적~\'' % (ob.han_iga(), obj.han_obj()))
            return
            
        msg = ''
        c = 0
        objs = copy.copy(obj.objs)
        for item in objs:
            if ob.getItemCount() >= getInt(MAIN_CONFIG['사용자아이템갯수']) or ob.getItemWeight() + item['무게'] > ob.getStr() * 10:
                if c == 0:
                    if obj.act == ACT_DEATH:
                        ob.sendLine('당신이 %s의 시체를 뒤집니다. \'뒤적~ 뒤적~\'' % obj.getNameA())
                        ob.sendRoom('%s %s의 시체를 뒤집니다. \'뒤적~ 뒤적~\'' % (ob.han_iga(), obj.getNameA()))
                    else:
                        ob.sendLine('당신이 %s 뒤집니다. \'뒤적~ 뒤적~\'' % obj.han_obj())
                        ob.sendRoom('%s %s 뒤집니다. \'뒤적~ 뒤적~\'' % (ob.han_iga(), obj.han_obj()))
                    return
                break

            c += 1
            obj.remove(item)
            ob.insert(item)
            if item.isOneItem():
                    ONEITEM.have(item.index, ob['이름'])
            if obj.act == ACT_DEATH:
                ob.sendLine('당신이 %s의 시체속에서 %s 뒤져서 가집니다.' % (obj.getNameA(), item.han_obj()))
                msg += '%s %s의 시체속에서 %s 뒤져서 가집니다.\r\n' %( ob.han_iga(), obj.getNameA(), item.han_obj())
            else:
                ob.sendLine('당신이 %s에게서 %s 뒤져서 가집니다.' % (obj.getNameA(), item.han_obj()))
                msg += '%s %s에게서 %s 뒤져서 가집니다.\r\n' %( ob.han_iga(), obj.getNameA(), item.han_obj())
        obj.timeofregen = time.time()
        ob.sendRoom(msg[:-2])


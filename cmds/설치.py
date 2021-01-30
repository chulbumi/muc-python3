# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [대상] 설치')
            return
        item = ob.findObjName(line)
        if item == None:
            ob.sendLine('☞ 그런 아이템이 소지품에 없어요.')
            return
        if item['종류'] != '설치아이템':
            ob.sendLine('☞ 설치할 수 있는 것이 아닙니다. ^^')
            return
        name = item['이름']
        owner = ob['이름']
        if ob.env['주인'] == '':
            if ob.env['방파주인'] == ob['소속']:
                if item.checkAttr('아이템속성', '공용보관함') == False:
                    ob.sendLine('☞ 이곳에 설치할 허가권이 없습니다.')
                    return
                owner = ob['소속']
            else:
                ob.sendLine('☞ 이곳에 설치할 허가권이 없습니다.')
                return
        elif ob.env['주인'] != ob['이름']:
            ob.sendLine('☞ 이곳에 설치할 허가권이 없습니다.')
            return
        if name in ob.env['설치리스트']:
            ob.sendLine('☞ 이미 설치가 되어 있습니다. ^^')
            return
        
        ob.env.setAttr('설치리스트', name)
        ob.env.save()
        
        box = Box()
        box['이름'] = item['이름']
        box['보관종류'] = item['보관종류']
        box['반응이름'] = item['반응이름']
        box['종류'] = item['종류']
        box['생성종류'] = item['생성종류']
        box['생성위치'] = item['생성위치']
        box['보관수량'] = item['보관수량']
        box['보관최대수량'] = item['보관최대수량']
        box['보관증가은전'] = item['보관증가은전']
        box['판매가격'] = item['판매가격']
        box['무게'] = item['무게']
        box['아이템속성'] = item['아이템속성']
        box['설명1'] = item['설명1']
        box['설명2'] = item['설명2']
        box['주인'] = owner
        box.index = '%s_%s' % (owner, item['이름'])
        box.path = 'data/box/%s.json' % box.index
        box.save()
        ob.env.insert(box)
        ob.sendLine('당신이 %s 설치합니다.' % item.han_obj())
        ob.sendRoom('%s %s 설치합니다.' % ( ob.han_iga(), item.han_obj() ))
        ob.remove(item)
        del item

